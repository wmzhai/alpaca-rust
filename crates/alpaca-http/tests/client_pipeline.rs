use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use alpaca_core::BaseUrl;
use alpaca_http::{Error, HttpClient, RequestParts, RetryConfig, StaticHeaderAuthenticator};
use reqwest::Method;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct MessageResponse {
    message: String,
}

#[derive(Clone)]
struct TestServer {
    base_url: String,
    retry_get_count: Arc<AtomicUsize>,
    retry_post_count: Arc<AtomicUsize>,
}

impl TestServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener addr should exist");
        let retry_get_count = Arc::new(AtomicUsize::new(0));
        let retry_post_count = Arc::new(AtomicUsize::new(0));
        let retry_get_count_task = Arc::clone(&retry_get_count);
        let retry_post_count_task = Arc::clone(&retry_post_count);

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let retry_get_count = Arc::clone(&retry_get_count_task);
                let retry_post_count = Arc::clone(&retry_post_count_task);

                tokio::spawn(async move {
                    let (method, path, headers, _body) = read_request(&mut stream).await;

                    let response = match path.as_str() {
                        "/json" => {
                            assert_eq!(headers.get("x-default-header"), Some(&"foundation".to_owned()));
                            assert_eq!(headers.get("apca-api-key-id"), Some(&"key-id".to_owned()));
                            http_response(
                                200,
                                &[ ("content-type", "application/json"), ("x-transport-id", "json-req") ],
                                r#"{"message":"ok"}"#,
                            )
                        }
                        "/text" => http_response(
                            200,
                            &[ ("content-type", "text/plain"), ("x-transport-id", "text-req") ],
                            "plain-text-body",
                        ),
                        "/empty" => http_response(
                            204,
                            &[ ("x-transport-id", "empty-req") ],
                            "",
                        ),
                        "/status" => http_response(
                            400,
                            &[ ("content-type", "text/plain"), ("x-transport-id", "status-req") ],
                            "bad request body",
                        ),
                        "/retry-json" => {
                            let attempt = retry_get_count.fetch_add(1, Ordering::SeqCst);
                            if attempt == 0 {
                                http_response(
                                    429,
                                    &[ ("retry-after", "0"), ("x-transport-id", "retry-1") ],
                                    "rate limited once",
                                )
                            } else {
                                http_response(
                                    200,
                                    &[ ("content-type", "application/json"), ("x-transport-id", "retry-2") ],
                                    r#"{"message":"retried"}"#,
                                )
                            }
                        }
                        "/retry-post" => {
                            assert_eq!(method, "POST");
                            retry_post_count.fetch_add(1, Ordering::SeqCst);
                            http_response(
                                429,
                                &[ ("retry-after", "0"), ("x-transport-id", "post-429") ],
                                "post should not retry",
                            )
                        }
                        _ => http_response(404, &[("content-type", "text/plain")], "not found"),
                    };

                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("response write should succeed");
                    let _ = stream.shutdown().await;
                });
            }
        });

        Self {
            base_url: format!("http://{}", address),
            retry_get_count,
            retry_post_count,
        }
    }

    fn base_url(&self) -> BaseUrl {
        BaseUrl::new(&self.base_url).expect("base url should parse")
    }
}

#[tokio::test]
async fn client_returns_json_text_and_no_content_responses_with_meta() {
    let server = TestServer::spawn().await;
    let client = HttpClient::builder()
        .default_header("x-default-header", "foundation")
        .expect("default header should parse")
        .request_id_header_name("x-transport-id")
        .expect("request id header should parse")
        .build()
        .expect("client should build");
    let auth = StaticHeaderAuthenticator::from_pairs([
        ("apca-api-key-id", "key-id"),
        ("apca-api-secret-key", "secret-key"),
    ])
    .expect("auth should build");
    let base_url = server.base_url();

    let json = client
        .send_json::<MessageResponse>(&base_url, RequestParts::new(Method::GET, "/json"), Some(&auth))
        .await
        .expect("json response should succeed");
    assert_eq!(json.body(), &MessageResponse { message: "ok".to_owned() });
    assert_eq!(json.meta().request_id(), Some("json-req"));

    let text = client
        .send_text(&base_url, RequestParts::new(Method::GET, "/text"), Some(&auth))
        .await
        .expect("text response should succeed");
    assert_eq!(text.body(), "plain-text-body");
    assert_eq!(text.meta().request_id(), Some("text-req"));

    let empty = client
        .send_no_content(&base_url, RequestParts::new(Method::DELETE, "/empty"), Some(&auth))
        .await
        .expect("no-content response should succeed");
    assert_eq!(empty.meta().request_id(), Some("empty-req"));
}

#[tokio::test]
async fn client_retries_get_on_429_but_not_post() {
    let server = TestServer::spawn().await;
    let client = HttpClient::builder()
        .request_id_header_name("x-transport-id")
        .expect("request id header should parse")
        .retry_config(
            RetryConfig::default()
                .with_retry_on_429(true)
                .with_respect_retry_after(true)
                .with_retryable_methods([Method::GET]),
        )
        .build()
        .expect("client should build");
    let base_url = server.base_url();

    let json = client
        .send_json::<MessageResponse>(&base_url, RequestParts::new(Method::GET, "/retry-json"), None)
        .await
        .expect("retrying get should succeed");
    assert_eq!(json.body(), &MessageResponse { message: "retried".to_owned() });
    assert_eq!(server.retry_get_count.load(Ordering::SeqCst), 2);

    let error = client
        .send_text(&base_url, RequestParts::new(Method::POST, "/retry-post"), None)
        .await
        .expect_err("post should not retry and should fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.request_id(), Some("post-429"));
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(server.retry_post_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn client_returns_error_meta_for_failed_http_status() {
    let server = TestServer::spawn().await;
    let client = HttpClient::builder()
        .request_id_header_name("x-transport-id")
        .expect("request id header should parse")
        .build()
        .expect("client should build");
    let base_url = server.base_url();

    let error = client
        .send_text(&base_url, RequestParts::new(Method::GET, "/status"), None)
        .await
        .expect_err("status endpoint should fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.request_id(), Some("status-req"));
            assert!(meta.body_snippet().is_some());
            assert!(meta.body_snippet().expect("body snippet should exist").contains("bad request"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> (String, String, HashMap<String, String>, Vec<u8>) {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end;

    loop {
        let read = stream.read(&mut chunk).await.expect("request read should succeed");
        if read == 0 {
            panic!("connection closed before request completed");
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = find_header_end(&buffer) {
            header_end = index;
            break;
        }
    }

    let header_bytes = &buffer[..header_end];
    let header_text = String::from_utf8(header_bytes.to_vec()).expect("headers should be utf8");
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().expect("request line should exist");
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next().expect("method should exist").to_owned();
    let path = request_line_parts.next().expect("path should exist").to_owned();
    let mut headers = HashMap::new();

    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').expect("header should contain colon");
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[(header_end + 4)..].to_vec();

    while body.len() < content_length {
        let read = stream.read(&mut chunk).await.expect("body read should succeed");
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    (method, path, headers, body)
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_response(status: u16, headers: &[(impl AsRef<str>, impl AsRef<str>)], body: &str) -> String {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Internal Server Error",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );

    for (name, value) in headers {
        response.push_str(name.as_ref());
        response.push_str(": ");
        response.push_str(value.as_ref());
        response.push_str("\r\n");
    }

    response.push_str("\r\n");
    response.push_str(body);
    response
}
