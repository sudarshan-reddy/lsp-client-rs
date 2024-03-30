use crate::protocol::ResponseMessage;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::fmt::Debug;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin {}
impl<T: AsyncRead + AsyncWrite + Unpin + ?Sized> AsyncReadWrite for T {}

type Stream = Pin<Box<dyn AsyncReadWrite + Send>>;

pub struct LspClient {
    stream: Stream,
}

impl LspClient {
    pub async fn new(addr: &str) -> Result<Self> {
        let scheme = addr.split(':').next().ok_or(anyhow!(
            "Invalid address format. Expected format: <scheme:address:port> or <scheme:path> for UNIX sockets."
        ))?;

        let stream: Stream = match scheme {
            "tcp" => {
                // Skip the scheme part and rejoin the rest (address and port)
                let addr = addr
                    .splitn(2, ':')
                    .nth(1)
                    .ok_or(anyhow!("Invalid TCP address format."))?;
                let tcp_stream = TcpStream::connect(addr).await?;
                Box::pin(tcp_stream) as Stream
            }
            "unix" => {
                // Skip the scheme part for UNIX domain socket path
                let path = addr
                    .splitn(2, ':')
                    .nth(1)
                    .ok_or(anyhow!("Invalid UNIX socket path format."))?;
                let unix_stream = UnixStream::connect(path).await?;
                Box::pin(unix_stream) as Stream
            }
            _ => {
                return Err(anyhow!(
                    "Unsupported scheme '{}'. Use 'tcp' or 'unix'.",
                    scheme
                ))
            }
        };

        Ok(Self { stream })
    }

    pub async fn send_request<T: Serialize + Debug>(&mut self, request: T) -> Result<()> {
        println!("Sending request: {:?}", request);
        let request_str = serde_json::to_string(&request)?;
        let content_length = request_str.len();
        let header = format!("Content-Length: {}\r\n\r\n{}", content_length, request_str);
        self.stream.write_all(header.as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn handle_response(&mut self) -> Result<ResponseMessage> {
        loop {
            let mut headers = Vec::new();
            let mut content_length: Option<usize> = None;

            // Read headers
            loop {
                let mut byte = [0];
                self.stream.read_exact(&mut byte).await?;
                headers.push(byte[0]);

                if headers.ends_with(b"\r\n\r\n") {
                    let headers_str = String::from_utf8_lossy(&headers);
                    for line in headers_str.lines() {
                        if line.starts_with("Content-Length:") {
                            let parts: Vec<&str> = line.splitn(2, ':').collect();
                            if parts.len() > 1 {
                                let length_str = parts[1].trim();
                                content_length = Some(length_str.parse()?);
                                break;
                            }
                        }
                    }
                    break; // Exit headers reading loop
                }
            }

            let content_length =
                content_length.ok_or_else(|| anyhow!("Failed to find Content-Length header"))?;
            let mut body = vec![0u8; content_length];
            self.stream.read_exact(&mut body).await?;
            println!("Response body: {:?}", String::from_utf8_lossy(&body));
            let response: ResponseMessage = serde_json::from_slice(&body)
                .map_err(|e| anyhow!("Failed to parse response body: {}", e))?;

            // If response has a valid id, return it
            if response.id.is_some() {
                return Ok(response);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::RequestMessage;
    use serde_json::json;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_send_request_and_response() {
        // Assume this is the exact request JSON your client will send
        let request = RequestMessage::new_initialize(
            1,
            std::process::id(),
            "file:///tmp".into(),
            "unit_test_client".into(),
            "0.1.0".into(),
            vec![],
        );

        let request_json = serde_json::to_string(&request).unwrap();
        let request_content_length = request_json.len();

        // The actual response from the server
        let response_payload = json!({
            "jsonrpc": "2.0",
            "id": 1, // Match the ID of the request
            "result": {}
        })
        .to_string();
        let response_content_length = response_payload.len();
        let server_response = format!(
            "Content-Length: {}\r\n\r\n{}",
            response_content_length, response_payload
        );

        // Set up the mock server
        let mock_server = Builder::new()
            .write(
                format!(
                    "Content-Length: {}\r\n\r\n{}",
                    request_content_length, request_json
                )
                .as_bytes(),
            )
            .read(server_response.as_bytes())
            .build();

        let mut lsp_client = LspClient {
            stream: Box::pin(mock_server),
        };

        // Test sending the request
        let send_result = lsp_client.send_request(request).await;
        assert!(send_result.is_ok());

        // Test handling the response
        let response = lsp_client.handle_response().await;
        assert!(response.is_ok());
        assert_eq!(response.unwrap().result.unwrap(), json!({}));
    }
}
