mod http;
mod socket;

use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use url::{Host, Url};

pub(crate) struct Timeouts {
    pub connect: Duration,
    pub read: Duration,
    pub request: Duration,
}

const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(30),
    read: Duration::from_secs(60),
    request: Duration::from_secs(300),
};

const USER_AGENT: &str = if cfg!(target_os = "macos") {
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_16_0) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
} else if cfg!(target_os = "windows") {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
} else {
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
};

struct GameHost(String);

impl GameHost {
    fn parse(host: &str) -> Option<Self> {
        let Host::Domain(host) = Host::parse(host).ok()? else {
            return None;
        };
        let host = host.strip_suffix('.').unwrap_or(&host);
        if !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) {
            return None;
        }
        ["aq.com", "artix.com"]
            .into_iter()
            .any(|root| {
                host == root
                    || host
                        .strip_suffix(root)
                        .is_some_and(|prefix| prefix.ends_with('.'))
            })
            .then(|| Self(host.to_owned()))
    }
}

pub(crate) struct Net {
    rt: tokio::runtime::Handle,
    http: reqwest::Client,
    cookies: std::sync::Arc<reqwest::cookie::Jar>,
    deadline: Duration,
    tasks: Mutex<Vec<tokio::task::AbortHandle>>,
    #[cfg(test)]
    routes: Vec<(&'static str, std::net::SocketAddr)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    Get,
    Post,
}

pub(crate) struct HttpRequest {
    pub method: Method,
    pub url: Url,
    pub headers: Vec<(String, String)>,
    pub body: Option<(Vec<u8>, String)>,
}

pub(crate) struct HttpResponse {
    pub url: Url,
    pub status: u16,
    pub content_type: Option<String>,
    pub length: Option<u64>,
    pub body: HttpBody,
}

#[derive(Clone)]
pub(crate) struct HttpBody {
    chunks: async_channel::Receiver<Vec<u8>>,
    failure: Arc<OnceLock<NetError>>,
}

impl HttpBody {
    pub(crate) async fn next_chunk(&self) -> Result<Option<Vec<u8>>, NetError> {
        match self.chunks.recv().await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(_) => match self.failure.get() {
                Some(error) => Err(error.clone()),
                None => Ok(None),
            },
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum NetError {
    #[error("{0}")]
    Http(String),
    #[error("the request timed out")]
    TimedOut,
    #[error("the net runtime is gone")]
    Shutdown,
}

impl From<reqwest::Error> for NetError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::TimedOut
        } else {
            Self::Http(error.to_string())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SocketEvent {
    Connected,
    Failed,
    TimedOut,
    Data(Vec<u8>),
    Closed,
}

impl Net {
    pub(crate) fn new(rt: tokio::runtime::Handle) -> Result<Net, NetError> {
        Self::build(rt, TIMEOUTS, reqwest::Client::builder())
    }

    #[cfg(test)]
    pub(crate) fn with_timeouts(
        rt: tokio::runtime::Handle,
        timeouts: Timeouts,
    ) -> Result<Net, NetError> {
        Self::build(rt, timeouts, reqwest::Client::builder().no_proxy())
    }

    fn build(
        rt: tokio::runtime::Handle,
        timeouts: Timeouts,
        builder: reqwest::ClientBuilder,
    ) -> Result<Net, NetError> {
        let http = builder
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(timeouts.connect)
            .read_timeout(timeouts.read)
            .build()?;
        Ok(Net {
            rt,
            http,
            cookies: Default::default(),
            deadline: timeouts.request,
            tasks: Mutex::new(Vec::new()),
            #[cfg(test)]
            routes: Vec::new(),
        })
    }

    #[cfg(test)]
    fn for_test(
        rt: tokio::runtime::Handle,
        routes: Vec<(&'static str, std::net::SocketAddr)>,
    ) -> Net {
        let mut builder = reqwest::Client::builder().no_proxy();
        for (host, addr) in &routes {
            builder = builder.resolve(host, *addr);
        }
        let mut net = Self::build(rt, TIMEOUTS, builder).unwrap();
        net.routes = routes;
        net
    }

    fn track(&self, task: tokio::task::JoinHandle<()>) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|task| !task.is_finished());
        tasks.push(task.abort_handle());
    }
}

impl Drop for Net {
    fn drop(&mut self) {
        for task in self.tasks.get_mut().unwrap().drain(..) {
            task.abort();
        }
    }
}
