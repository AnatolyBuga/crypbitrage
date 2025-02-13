use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helps parsing cli args.
/// Part of the binary, since it has nothing to do with the library
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// OKX INSTRUMENT NAME
    #[arg(short, long, value_name = "OKX_INST")]
    okx_inst: String,

    // DERIBIT INSTRUMENT NAME
    // #[arg(short, long, value_name = "DERIBIT_INST")]
    // deribit_inst: String,
}

const OKX_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";
const DERIBIT_URL: &str = "wss://www.deribit.com/ws/api/v2";

#[tokio::main]
async fn main() {

    let cli = CliArgs::parse();

    let subscribe_msg = json!({
        "op": "subscribe",
        "args": [
            {"channel": "books", "instId": cli.okx_inst}
        ]
    }).to_string();

    let (ws_stream, _) = connect_async(OKX_URL).await.expect("WebSocket connection failed");
    let (mut write, mut read) = ws_stream.split();

    // Send subscription request
    write.send(Message::Text(subscribe_msg.into()))
        .await
        .expect("Failed to send subscription message");

    println!("Subscribed to {} order book.", cli.okx_inst);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);
            }
            Ok(_) => (),
            Err(e) => {
                eprintln!("WebSocket error: {:?}", e);
                break;
            }
        }
    }

    println!("WebSocket connection closed.");
    
}

