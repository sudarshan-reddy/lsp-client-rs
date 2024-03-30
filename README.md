
# lsp-rs

`lsp-rs` is a Rust library designed to facilitate communication with Language Server Protocol (LSP) servers. It provides an asynchronous client for sending and receiving messages in accordance with the LSP specification, making it easier to integrate LSP features into Rust applications.

## Features

- Supports both TCP and Unix Domain Socket connections to LSP servers.
- Serialization and deserialization of LSP requests and responses.
- Includes structures for commonly used LSP messages such as `Initialize`, `Notification`, and `Response`.
- Supports Go to defintion.

## Installation

Add `lsp-rs` to your `Cargo.toml`:

```toml
[dependencies]
lsp-rs = "0.1.0"
```

## Usage

Here's a basic example of how to use lsp-rs to initialize a connection with an LSP server and send an initialization request:

```rust
use lsp_rs::{LspClient, RequestMessage};
use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let mut client = LspClient::new("tcp:127.0.0.1:8080").await?;
        let initialize_request = RequestMessage::new_initialize(
            1, // Request ID
            std::process::id(),
            "file:///path/to/workspace".into(),
            "MyLSPClient".into(),
            "1.0".into(),
            vec![], // Workspace folders
        );
        client.send_request(initialize_request).await?;
        let response = client.handle_response().await?;
        println!("Received response: {:?}", response);
        Ok(())
    })
}
```

## Limitations

- I've only tested this with gopls. 
- Currently, lsp-rs supports a subset of the LSP specification. Additional request and response types may need to be implemented based on your requirements.
- The library is designed for basic LSP interactions; complex workflows involving advanced LSP features are not yet supported.

## Future Work
- Implement the full range of LSP requests and responses.
- Enhance error handling and logging for better debugging and reliability.
- Add support for more complex LSP features like incremental synchronization and workspace updates.
- Add Notifcation handling.
