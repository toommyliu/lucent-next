use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use lucent_host::projection::Projection;
use lucent_protocol::{Packet, PacketEncoding};
use serde_json::{Value, json};
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<BTreeMap<String, Value>>>>);

struct Fields(BTreeMap<String, Value>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{value:?}").into());
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().into(), value.into());
    }
}

impl<S: Subscriber> Layer<S> for Capture {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = Fields(BTreeMap::new());
        event.record(&mut fields);
        fields
            .0
            .insert("target".into(), event.metadata().target().into());
        fields
            .0
            .insert("level".into(), event.metadata().level().as_str().into());
        self.0.lock().unwrap().push(fields.0);
    }
}

#[test]
fn rejected_entries_log_structured_context_without_account_data() {
    let capture = Capture::default();
    let subscriber = Registry::default().with(capture.clone());
    let mut projection = Projection::default();
    projection.auth.username = "account-canary".into();
    projection.auth.password = "credential-canary".into();
    let packet = Packet::Extension {
        command: "initInventory".into(),
        data: json!({"items":[{"ItemID":0,"sName":"private-name-canary"}],"password":"credential-canary"}),
        raw: "raw-packet-canary".into(),
        encoding: PacketEncoding::Json,
    };
    tracing::subscriber::with_default(subscriber, || {
        projection.packet(&packet, 0.0, false);
    });
    let captured = capture.0.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0]["level"], "WARN");
    assert_eq!(captured[0]["command"], "initInventory");
    assert_eq!(captured[0]["rejected"], 1);
    assert_eq!(captured[0]["container"], "Inventory");
    assert_eq!(captured[0]["target"], "lucent_host::projection::baselines");
    let serialized = serde_json::to_string(&*captured).unwrap();
    for sensitive in [
        "account-canary",
        "credential-canary",
        "private-name-canary",
        "raw-packet-canary",
    ] {
        assert!(!serialized.contains(sensitive));
        assert!(!format!("{:?}", projection.auth).contains(sensitive));
    }
    assert!(!serialized.contains("operation"));
    assert!(!serialized.contains("phase"));
}
