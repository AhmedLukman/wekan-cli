use std::time::Duration;

use reqwest::StatusCode;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use super::{ClientError, MAX_RESPONSE_BYTES, ResponseBody};
use crate::client::{ServerUrl, WekanClientFactory};

#[derive(Clone, Copy, Debug)]
enum Framing {
    Length,
    Chunked,
    Close,
    TruncatedLength,
    TruncatedChunked,
}

async fn fetch(
    status: StatusCode,
    framing: Framing,
    body: Vec<u8>,
    success_limit: usize,
) -> Result<ResponseBody, ClientError> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|end| end == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let length = socket.read(&mut buffer).await.unwrap();
            assert!(length > 0 && request.len() < 16384);
            request.extend_from_slice(&buffer[..length]);
        }
        let mut headers = format!(
            "HTTP/1.1 {} Test\r\nConnection: close\r\nRetry-After: 17\r\n",
            status.as_u16()
        );
        match framing {
            Framing::Length => headers.push_str(&format!("Content-Length: {}\r\n", body.len())),
            Framing::TruncatedLength => {
                headers.push_str(&format!("Content-Length: {}\r\n", body.len() + 1))
            }
            Framing::Chunked | Framing::TruncatedChunked => {
                headers.push_str("Transfer-Encoding: chunked\r\n")
            }
            Framing::Close => {}
        }
        headers.push_str("\r\n");
        socket.write_all(headers.as_bytes()).await.unwrap();
        // A size-limit rejection can close the connection before the fixture
        // finishes writing. Other tests assert the complete returned bytes.
        if matches!(framing, Framing::TruncatedChunked) {
            let _ = socket
                .write_all(format!("{:x}\r\n", body.len() + 1).as_bytes())
                .await;
            let _ = socket.write_all(&body).await;
        } else {
            // Keep the final byte in a separate write / HTTP chunk so overflow
            // is tested after accumulating a response at its exact limit.
            let split = body.len().saturating_sub(1);
            for part in [&body[..split], &body[split..]]
                .into_iter()
                .filter(|part| !part.is_empty())
            {
                if matches!(framing, Framing::Chunked)
                    && socket
                        .write_all(format!("{:x}\r\n", part.len()).as_bytes())
                        .await
                        .is_err()
                {
                    return;
                }
                if socket.write_all(part).await.is_err() {
                    return;
                }
                if matches!(framing, Framing::Chunked) && socket.write_all(b"\r\n").await.is_err() {
                    return;
                }
            }
            if matches!(framing, Framing::Chunked) {
                let _ = socket.write_all(b"0\r\n\r\n").await;
            }
        }
    });
    let client = WekanClientFactory::for_server(ServerUrl::parse(&url).unwrap(), false)
        .create()
        .unwrap();
    let result = client
        .execute_request_with_limit(
            client.http.get(url).timeout(Duration::from_secs(3)),
            success_limit,
        )
        .await;
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    result
}

#[tokio::test]
async fn exact_and_smaller_bodies_succeed_for_all_http_framings() {
    for framing in [Framing::Length, Framing::Chunked, Framing::Close] {
        for length in [0, 31, 32] {
            let body = vec![b'x'; length];
            let response = fetch(StatusCode::OK, framing, body.clone(), 32)
                .await
                .unwrap();
            assert_eq!(response.body, body, "{framing:?}, {length} bytes");
            assert_eq!(response.status, StatusCode::OK);
            assert_eq!(response.retry_after_seconds, Some(17));
        }
    }
}

#[tokio::test]
async fn streaming_overflow_preserves_status_retry_after_and_the_effective_limit() {
    for framing in [Framing::Length, Framing::Chunked, Framing::Close] {
        for status in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
            let limit = if status == StatusCode::OK {
                32
            } else {
                MAX_RESPONSE_BYTES
            };
            let error = fetch(status, framing, vec![b'x'; limit + 1], 32)
                .await
                .err()
                .unwrap();
            let ClientError::ResponseTooLarge {
                limit_bytes,
                status: actual_status,
                retry_after_seconds,
            } = error
            else {
                panic!("expected size rejection for {framing:?}: {error:?}");
            };
            assert_eq!(limit_bytes, limit);
            assert_eq!(actual_status, status);
            assert_eq!(retry_after_seconds, Some(17));
        }
    }
}

#[tokio::test]
async fn error_bodies_use_the_shared_limit_instead_of_a_smaller_success_limit() {
    for framing in [Framing::Length, Framing::Chunked, Framing::Close] {
        let body = vec![b'x'; MAX_RESPONSE_BYTES];
        let response = fetch(StatusCode::TOO_MANY_REQUESTS, framing, body.clone(), 32)
            .await
            .unwrap();
        assert_eq!(response.body, body);
        let error = response.require_status(StatusCode::OK).unwrap_err();
        assert!(matches!(
            error,
            ClientError::Server {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_after_seconds: Some(17),
                ..
            }
        ));
    }
}

#[tokio::test]
async fn interrupted_bodies_preserve_status_and_retry_after() {
    for framing in [Framing::TruncatedLength, Framing::TruncatedChunked] {
        for status in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
            let error = fetch(status, framing, b"partial".to_vec(), 32)
                .await
                .err()
                .unwrap();
            assert!(
                matches!(error, ClientError::ResponseBody {
                status: actual_status, retry_after_seconds: Some(17), ..
            } if actual_status == status),
                "{framing:?}: {error:?}"
            );
        }
    }
}
