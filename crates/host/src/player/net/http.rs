use reqwest::cookie::CookieStore;
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};

use super::*;

const REDIRECT_LIMIT: usize = 10;

fn http_url(url: &Url) -> Result<(), NetError> {
    if matches!(url.scheme(), "http" | "https") {
        Ok(())
    } else {
        Err(NetError::Http(
            "only HTTP and HTTPS URLs are allowed".into(),
        ))
    }
}

fn is_game(url: &Url) -> bool {
    url.host_str().and_then(GameHost::parse).is_some()
}

fn headers(values: Vec<(String, String)>) -> Result<HeaderMap, NetError> {
    let mut headers = HeaderMap::new();
    for (name, value) in values {
        let name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|e| NetError::Http(e.to_string()))?;
        let value = HeaderValue::from_str(&value).map_err(|e| NetError::Http(e.to_string()))?;
        headers.append(name, value);
    }
    Ok(headers)
}

impl Net {
    pub(crate) fn fetch(
        &self,
        request: HttpRequest,
    ) -> impl Future<Output = Result<HttpResponse, NetError>> + 'static {
        let http = self.http.clone();
        let cookies = self.cookies.clone();
        let deadline = tokio::time::Instant::now() + self.deadline;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.track(self.rt.spawn(async move {
            let mut response =
                match tokio::time::timeout_at(deadline, send(&http, &cookies, request)).await {
                    Ok(Ok(response)) => response,
                    Ok(Err(error)) => {
                        let _ = tx.send(Err(error));
                        return;
                    }
                    Err(_) => {
                        let _ = tx.send(Err(NetError::TimedOut));
                        return;
                    }
                };
            let (chunks_tx, chunks_rx) = async_channel::bounded(16);
            let failure = Arc::new(OnceLock::new());
            let head = HttpResponse {
                url: response.url().clone(),
                status: response.status().as_u16(),
                content_type: response
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned),
                length: response.content_length(),
                body: HttpBody {
                    chunks: chunks_rx,
                    failure: failure.clone(),
                },
            };
            if tx.send(Ok(head)).is_err() {
                return;
            }
            loop {
                let chunk = tokio::time::timeout_at(deadline, response.chunk()).await;
                let chunk = match chunk {
                    Ok(Ok(Some(chunk))) => chunk.to_vec(),
                    Ok(Ok(None)) => break,
                    Ok(Err(error)) => {
                        let _ = failure.set(error.into());
                        break;
                    }
                    Err(_) => {
                        let _ = failure.set(NetError::TimedOut);
                        break;
                    }
                };
                match tokio::time::timeout_at(deadline, chunks_tx.send(chunk)).await {
                    Ok(Ok(())) => {}
                    Ok(_) => break,
                    Err(_) => {
                        let _ = failure.set(NetError::TimedOut);
                        break;
                    }
                }
            }
        }));
        async move { rx.await.unwrap_or(Err(NetError::Shutdown)) }
    }
}

async fn send(
    http: &reqwest::Client,
    cookies: &reqwest::cookie::Jar,
    request: HttpRequest,
) -> Result<reqwest::Response, NetError> {
    let HttpRequest {
        mut method,
        mut url,
        headers: values,
        mut body,
    } = request;
    let mut headers = headers(values)?;
    for hop in 0..=REDIRECT_LIMIT {
        http_url(&url)?;
        let game = is_game(&url);
        let mut hop_headers = headers.clone();
        hop_headers.remove(header::USER_AGENT);
        hop_headers.remove("artixmode");
        if game {
            hop_headers.insert(header::USER_AGENT, HeaderValue::from_static(USER_AGENT));
            hop_headers.insert("artixmode", HeaderValue::from_static("launcher"));
            if !hop_headers.contains_key(header::COOKIE)
                && let Some(value) = cookies.cookies(&url)
            {
                hop_headers.insert(header::COOKIE, value);
            }
        } else {
            hop_headers.remove(header::COOKIE);
        }
        let mut builder = http
            .request(
                match method {
                    Method::Get => reqwest::Method::GET,
                    Method::Post => reqwest::Method::POST,
                },
                url.clone(),
            )
            .headers(hop_headers);
        if let Some((bytes, mime)) = &body {
            builder = builder
                .header(header::CONTENT_TYPE, mime)
                .body(bytes.clone());
        }
        let response = builder.send().await?;
        if game {
            cookies.set_cookies(
                &mut response.headers().get_all(header::SET_COOKIE).iter(),
                &url,
            );
        }
        let status = response.status().as_u16();
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Ok(response);
        }
        let Some(location) = response.headers().get(header::LOCATION) else {
            return Ok(response);
        };
        if hop == REDIRECT_LIMIT {
            return Err(NetError::Http("too many HTTP redirects".into()));
        }
        let location = location
            .to_str()
            .map_err(|e| NetError::Http(e.to_string()))?;
        let next = url
            .join(location)
            .map_err(|e| NetError::Http(e.to_string()))?;
        http_url(&next)?;
        if next.origin() != url.origin() {
            for name in [
                header::AUTHORIZATION,
                header::PROXY_AUTHORIZATION,
                header::COOKIE,
                header::HOST,
                header::ORIGIN,
                header::REFERER,
            ] {
                headers.remove(name);
            }
        }
        if status == 303 || (matches!(status, 301 | 302) && method == Method::Post) {
            method = Method::Get;
            body = None;
            for name in [
                header::CONTENT_TYPE,
                header::CONTENT_LENGTH,
                header::CONTENT_ENCODING,
                header::CONTENT_LANGUAGE,
                header::CONTENT_LOCATION,
                header::TRANSFER_ENCODING,
            ] {
                headers.remove(name);
            }
        }
        url = next;
    }
    unreachable!()
}
