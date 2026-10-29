use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::{
    bridge::{Error, ScriptEvent, Value},
    player::{Inbox, Msg},
    projection::{
        Projection,
        wire::{wire_bool, wire_i64, wire_int},
    },
};
use lucent_protocol::{
    Packet, PacketEncoding,
    item::{ItemContainer, ItemData},
    operation::{Operation, OperationAnswer},
};
use parking_lot::{Mutex, RwLock};

const CALL_TIMEOUT: Duration = Duration::from_secs(5);
const PACKET_TIMEOUT: Duration = Duration::from_secs(10);

type Reply = mpsc::Sender<Result<AcceptResponse, Error>>;

#[derive(Default)]
pub(crate) struct Packets(Mutex<PacketState>);
#[derive(Default)]
struct PacketState {
    stopped: bool,
    waiting: Option<(i64, Reply)>,
}

struct AcceptResponse {
    success: bool,
    bank: Option<bool>,
    house: Option<bool>,
    quantity: Option<f64>,
    quantity_now: Option<f64>,
}

impl Packets {
    fn subscribe(&self, item: i64) -> Result<mpsc::Receiver<Result<AcceptResponse, Error>>, Error> {
        let mut state = self.0.lock();
        if state.stopped {
            return Err(Error::Stopped);
        }
        let (reply, receiver) = mpsc::channel();
        state.waiting = Some((item, reply));
        Ok(receiver)
    }

    pub(crate) fn packet(&self, packet: &Packet) {
        let mut state = self.0.lock();
        let Some((id, _)) = &state.waiting else {
            return;
        };
        let Packet::Extension {
            command,
            data,
            encoding: PacketEncoding::Json,
            ..
        } = packet
        else {
            return;
        };
        if command != "getDrop" {
            return;
        }
        let decode = || {
            let object = data.as_object()?;
            let item = wire_i64(object.get("ItemID")?).filter(|id| *id > 0)?;
            if item != *id {
                return None;
            }
            let optional = |key: &str, decode: fn(&serde_json::Value) -> Option<bool>| {
                object
                    .get(key)
                    .map_or(Some(None), |value| decode(value).map(Some))
            };
            let quantity = |key: &str| {
                object
                    .get(key)
                    .map_or(Some(None), |value| wire_int(value).map(Some))
            };
            Some(AcceptResponse {
                success: wire_bool(object.get("bSuccess")?)?,
                bank: optional("bBank", wire_bool)?,
                house: optional("bHouse", wire_bool)?,
                quantity: quantity("iQty")?,
                quantity_now: quantity("iQtyNow")?,
            })
        };
        if let Some(response) = decode() {
            let (_, reply) = state.waiting.take().unwrap();
            let _ = reply.send(Ok(response));
        }
    }

    pub(crate) fn invalidate(&self) {
        self.0.lock().waiting.take();
    }

    pub(crate) fn stop(&self) {
        let mut state = self.0.lock();
        state.stopped = true;
        if let Some((_, reply)) = state.waiting.take() {
            let _ = reply.send(Err(Error::Stopped));
        }
    }
}

#[derive(Clone)]
pub(crate) struct Endpoint {
    pub(crate) inbox: Inbox,
    pub(crate) projection: Arc<RwLock<Projection>>,
    pub(crate) stopped: Arc<AtomicBool>,
    pub(crate) packets: Arc<Packets>,
}

impl Endpoint {
    fn read<R>(&self, read: impl FnOnce(&Projection) -> R) -> Result<R, Error> {
        self.check()?;
        let projection = self
            .projection
            .try_read_for(CALL_TIMEOUT)
            .ok_or(Error::Timeout)?;
        self.check()?;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read(&projection)))
            .map_err(|_| Error::ReadPanicked)
    }
    fn check(&self) -> Result<(), Error> {
        if self.stopped.load(Ordering::Acquire) {
            Err(Error::Stopped)
        } else {
            Ok(())
        }
    }
    fn accept(&self, selector: lucent_protocol::query::ItemSelector) -> Result<bool, Error> {
        self.check()?;
        let (reply, logged_in) = crate::bridge::Pending::channel();
        self.inbox.post(Msg::Command(
            lucent_protocol::command::Command::RefreshLoggedIn {},
            reply,
        ));
        let logged_in = match logged_in.recv_timeout(CALL_TIMEOUT) {
            Ok(value) => serde_json::from_str::<bool>(&value).unwrap_or(false),
            Err(Error::Stopped) => return Err(Error::Stopped),
            Err(_) => self.read(|projection| projection.auth.logged_in)?,
        };
        if !logged_in {
            return Ok(false);
        }
        let prepared = self.read(|projection| {
            let drop = projection
                .items
                .resolve(ItemContainer::Drop, &selector)?
                .data()
                .clone();
            let quantities = [
                ItemContainer::Bank,
                ItemContainer::House,
                ItemContainer::Inventory,
            ]
            .map(|container| {
                projection
                    .items
                    .get(container, drop.item_id)
                    .map_or(0.0, |item| item.data().quantity)
            });
            let response = self.packets.subscribe(drop.item_id);
            Some((drop, quantities, response))
        })?;
        let Some((drop, quantities, response)) = prepared else {
            return Ok(false);
        };
        let response = response?;
        let result = self.accept_subscribed(&drop, quantities, response);
        self.packets.invalidate();
        result
    }

    fn accept_subscribed(
        &self,
        drop: &ItemData,
        before: [f64; 3],
        response: mpsc::Receiver<Result<AcceptResponse, Error>>,
    ) -> Result<bool, Error> {
        let deadline = Instant::now() + PACKET_TIMEOUT;
        let call = self
            .inbox
            .call_loader("drops.accept", vec![Value::Number(drop.item_id as f64)])
            .recv_timeout(CALL_TIMEOUT);
        self.check()?;
        if !matches!(call, Ok(Value::Bool(true))) {
            return Ok(false);
        }
        let response =
            match response.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Ok(response)) => response,
                Ok(Err(Error::Stopped)) => return Err(Error::Stopped),
                _ => {
                    self.check()?;
                    return Ok(false);
                }
            };
        if !response.success {
            return Ok(false);
        }
        let (container, quantity) = if response.bank == Some(true) && drop.category != "Class" {
            (ItemContainer::Bank, before[0])
        } else if response.house == Some(true) || drop.house_item {
            (ItemContainer::House, before[1])
        } else {
            (ItemContainer::Inventory, before[2])
        };
        let expected = response
            .quantity_now
            .unwrap_or(quantity + response.quantity.unwrap_or(drop.quantity));
        self.read(|projection| {
            projection
                .items
                .get(container, drop.item_id)
                .is_some_and(|item| item.data().quantity >= expected)
        })
    }
}

pub(crate) struct Request {
    pub(crate) id: u64,
    pub(crate) operation: Operation,
    pub(crate) player: Endpoint,
}

pub(crate) struct Operations {
    pub(crate) sender: mpsc::SyncSender<Option<Request>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Operations {
    pub(crate) fn start(id: u64) -> std::io::Result<Self> {
        let (sender, requests) =
            mpsc::sync_channel::<Option<Request>>(crate::bridge::EVENT_CAPACITY);
        let thread = thread::Builder::new()
            .name(format!("operations-{id}"))
            .spawn(move || {
                while let Ok(Some(request)) = requests.recv() {
                    let answer = match request.operation {
                        Operation::AcceptDrop { selector } => request
                            .player
                            .accept(selector)
                            .map(OperationAnswer::AcceptDrop),
                    };
                    request
                        .player
                        .inbox
                        .post(Msg::OperationAnswer(ScriptEvent::Operation {
                            id: request.id,
                            answer,
                        }));
                }
            })?;
        Ok(Self {
            sender,
            thread: Some(thread),
        })
    }
}

impl Drop for Operations {
    fn drop(&mut self) {
        let _ = self.sender.send(None);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
