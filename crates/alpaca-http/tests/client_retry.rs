use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use alpaca_core::BaseUrl;
use alpaca_http::{HttpClient, RequestParts, RetryConfig};
use reqwest::Method;
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::test]
async fn get_json_retries_body_read_failures() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
    let addr = listener.local_addr().expect("listener should have address");
    let server_attempts = attempts.clone();

    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.expect("request should arrive");
            let attempt = server_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            handle_request(stream, attempt).await;
        }
    });

    let client = HttpClient::builder()
        .retry_config(RetryConfig::default().with_max_retries(1))
        .build()
        .expect("client should build");
    let base_url = BaseUrl::new(format!("http://{}", addr)).expect("valid base url");
    let request = RequestParts::new(Method::GET, "/probe").with_operation("test.body_retry");

    let response = client
        .send_json::<Value>(&base_url, request, None)
        .await
        .expect("GET body read failures should be retried");

    assert_eq!(response.body()["ok"], true);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.await.expect("server task should finish");
}

async fn handle_request(mut stream: TcpStream, attempt: usize) {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await;

    if attempt == 1 {
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 128\r\n\r\n{\"ok\"",
            )
            .await
            .expect("truncated response should write");
        return;
    }

    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}",
        )
        .await
        .expect("valid response should write");
}
