use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

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

pub(crate) struct Net {
    rt: tokio::runtime::Handle,
    http: reqwest::Client,
    tasks: Mutex<Vec<tokio::task::AbortHandle>>,
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
    pub body: async_channel::Receiver<Result<Vec<u8>, NetError>>,
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
        Net::with_timeouts(rt, TIMEOUTS)
    }

    pub(crate) fn with_timeouts(
        rt: tokio::runtime::Handle,
        timeouts: Timeouts,
    ) -> Result<Net, NetError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "artixmode",
            reqwest::header::HeaderValue::from_static("launcher"),
        );
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .cookie_store(true)
            .connect_timeout(timeouts.connect)
            .read_timeout(timeouts.read)
            .timeout(timeouts.request)
            .build()?;
        Ok(Net {
            rt,
            http,
            tasks: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn fetch(
        &self,
        request: HttpRequest,
    ) -> impl Future<Output = Result<HttpResponse, NetError>> + 'static {
        let http = self.http.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.track(self.rt.spawn(async move {
            let method = match request.method { Method::Get => reqwest::Method::GET, Method::Post => reqwest::Method::POST };
            let mut builder = http.request(method, request.url);
            for (name, value) in request.headers { builder = builder.header(name, value); }
            if let Some((body, mime)) = request.body {
                builder = builder.header(reqwest::header::CONTENT_TYPE, mime).body(body);
            }
            let mut response = match builder.send().await {
                Ok(response) => response,
                Err(error) => { let _ = tx.send(Err(error.into())); return; }
            };
            tracing::debug!(url = %response.url(), status = response.status().as_u16(), "HTTP response");
            let (chunks_tx, chunks_rx) = async_channel::bounded(16);
            let head = HttpResponse {
                url: response.url().clone(), status: response.status().as_u16(),
                content_type: response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_owned),
                length: response.content_length(), body: chunks_rx,
            };
            if tx.send(Ok(head)).is_err() { return; }
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => if chunks_tx.send(Ok(chunk.to_vec())).await.is_err() { break; },
                    Ok(None) => break,
                    Err(error) => { let _ = chunks_tx.send(Err(error.into())).await; break; }
                }
            }
        }));
        async move { rx.await.unwrap_or(Err(NetError::Shutdown)) }
    }

    pub(crate) fn connect(
        &self,
        host: String,
        port: u16,
        timeout: Duration,
        outgoing: async_channel::Receiver<Vec<u8>>,
        mut deliver: impl FnMut(SocketEvent) -> bool + Send + 'static,
    ) {
        self.track(self.rt.spawn(async move {
            let stream = match tokio::time::timeout(
                timeout,
                tokio::net::TcpStream::connect((host.as_str(), port)),
            )
            .await
            {
                Ok(Ok(stream)) => stream,
                Ok(Err(error)) => {
                    tracing::warn!(%host, port, %error, "socket connection failed");
                    deliver(SocketEvent::Failed);
                    return;
                }
                Err(_) => {
                    deliver(SocketEvent::TimedOut);
                    return;
                }
            };
            if !deliver(SocketEvent::Connected) {
                return;
            }
            let (mut read, mut write) = stream.into_split();
            let reading = async {
                let mut bytes = [0; 4096];
                loop {
                    match read.read(&mut bytes).await {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            if !deliver(SocketEvent::Data(bytes[..count].to_vec())) {
                                break;
                            }
                        }
                    }
                }
            };
            let writing = async {
                while let Ok(bytes) = outgoing.recv().await {
                    if write.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                let _ = write.shutdown().await;
            };
            tokio::select! { _ = reading => {}, _ = writing => {} }
            deliver(SocketEvent::Closed);
        }));
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    fn request_headers() -> Vec<(String, String)> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = Url::parse(&format!(
            "http://{}/game.swf",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut buf = [0; 4096];
            while !head.ends_with(b"\r\n\r\n") {
                let read = stream.read(&mut buf).unwrap();
                head.extend_from_slice(&buf[..read]);
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
            String::from_utf8(head)
                .unwrap()
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(": "))
                .map(|(name, value)| (name.to_ascii_lowercase(), value.to_owned()))
                .collect()
        });
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let net = Net::new(runtime.handle().clone()).unwrap();
        let request = HttpRequest {
            method: Method::Get,
            url,
            headers: Vec::new(),
            body: None,
        };
        runtime.block_on(async {
            let response = net.fetch(request).await.unwrap();
            while response.body.recv().await.is_ok() {}
        });
        server.join().unwrap()
    }

    #[test]
    fn a_request_carries_the_launcher_user_agent_and_mode() {
        let headers = request_headers();
        let header = |name: &str| {
            headers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, value)| value.as_str())
        };
        let user_agent = if cfg!(target_os = "macos") {
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_16_0) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
        } else if cfg!(target_os = "windows") {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
        } else {
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
        };
        assert_eq!(header("user-agent"), Some(user_agent));
        assert_eq!(header("artixmode"), Some("launcher"));
    }
}
