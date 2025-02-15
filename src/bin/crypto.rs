use std::collections::BTreeSet;

use clap::Parser;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use serde::{Deserialize, Serialize, Deserializer};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helps parsing cli args.
/// Part of the binary, since it has nothing to do with the library
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// OKX INSTRUMENT NAME
    #[arg(short, long, value_name = "OKX_INST", default_value="BTC-USD-251226-100000-C")]
    okx_inst: String,

    /// DERIBIT INSTRUMENT NAME
    #[arg(short, long, value_name = "DERIBIT_INST", default_value="BTC-26DEC25-100000-C")]
    deribit_inst: String,
}

const OKX_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";
const DERIBIT_URL: &str = "wss://www.deribit.com/ws/api/v2";
type Price = f64;
type Quantity = f64;

enum OrderType{
    Bid,
    Ask
}

struct Order(Price, Quantity, OrderType);

struct OrderFromExchange{
    order: Order,
    exchange_id: u8,
}

struct ExchangeConnection<S>{
    /// Exchange id
    id: u8,
    /// Websocket read stream
    read: SplitStream<S>,
    // TODO
    // write: 
}

// impl<S> ExchangeConnection<S> {
//     pub fn from_url(id: u8, url: &str, subscribe_msg: String) -> Self {

//         let (ws_stream, _) = connect_async(OKX_URL).await.expect("WebSocket connection failed");
//         let (mut write, mut read) = ws_stream.split();

//         tokio::spawn(  async  {

//         });
//     }
// }


struct ExchangeConnectionHandle;

// struct ExchangePool {
//     ex1: ExchangeConnection,
//     ex2: ExchangeConnection,
// }

struct CrossExchangeLOB{
    bids: BTreeSet<OrderFromExchange>,
    asks: BTreeSet<OrderFromExchange>
}

#[derive(Serialize, Deserialize, Debug)]
struct OkxBidsAsks{
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    asks: Vec<[f64;2]>,
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    bids: Vec<[f64;2]>
}

#[derive(Serialize, Deserialize, Debug)]
struct OkxData{
    data: Vec<OkxBidsAsks>
}

// Custom deserializer function
fn deserialize_vec_of_arrays<'de, D>(deserializer: D) -> Result<Vec<[f64; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Vec<Vec<String>> = Deserialize::deserialize(deserializer)?;

    raw.into_iter()
        .map(|arr| {
            let parsed: Result<Vec<f64>, _> = arr.iter().take(2).map(|s| s.parse::<f64>()).collect();
            parsed
                .map_err(serde::de::Error::custom)?
                .try_into()
                .map_err(|_| serde::de::Error::custom("Failed to convert to [f64; 2]"))
        })
        .collect()
}

#[derive(Debug)]
enum ExchangeData{
    OKX(OkxData),
}

async fn arbitrage(mut receiver: UnboundedReceiver<ExchangeData>){
    while let Some(msg) = receiver.recv().await {
        println!("{msg:#?}")
    }
}

#[tokio::main]
async fn main() {

    let cli = CliArgs::parse();

    // TODO Not sure about channel size
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    // OKX
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

    println!("Subscribed to OKX");

    // DERIBIT_URL

    let channel = format!("book.{}.none.20.100ms", cli.deribit_inst);

    let subscribe_msg2 = json!({
        "method": "public/subscribe",
        "params": {"channels": [channel]},
        "jsonrpc": "2.0",
        "id": 0}).to_string();

    let (ws_stream2, _) = connect_async(DERIBIT_URL).await.expect("WebSocket connection failed");
    let (mut write2, mut read2) = ws_stream2.split();

    // Send subscription request
    write2.send(Message::Text(subscribe_msg2.into()))
        .await
        .expect("Failed to send subscription message");

    println!("Subscribed to DERIBIT");


    let task1 = tokio::spawn( async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("OKX: {}", text);
                    // TODO Optimisation decode only data , not the whole message
                    match serde_json::from_slice::<OkxData>(text.as_bytes()) {
                        Ok(okx_data) => sender.send(ExchangeData::OKX(okx_data)).expect("OKX couldn't send"),//println!("OKX Deser: {okx_data:#?}"),
                        Err(e) => println!("OKX: couldn't deserialise: {e}")
                        
                    } 
                    // let mut json: Value = serde_json::from_slice(text.as_bytes()).expect("Invalid JSON");
                    // if let Some(_) = json.get_mut("data") {
                    //     let data = json["data"].take();
                    //     println!("OKX Extracted data: {:?}", data);
                    //     let okx_data: OkxData = serde_json::from_value(data).expect("Failed to deser OKX Data");
                    //     println!("OKX Deser data: {:?}", okx_data);
                    // } else {
                    //     println!("OKX Data Not found");
                    // }
                    // 
                }
                Ok(_) => (),
                Err(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    });

    let task2 = tokio::spawn( async move {
        while let Some(msg) = read2.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("Received DERIBIT: {}", text);
                }
                Ok(_) => (),
                Err(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    });

    let task3 = tokio::task::spawn_blocking( || {arbitrage(receiver)} );

    // https://stackoverflow.com/questions/69638710/when-should-you-use-tokiojoin-over-tokiospawn
    tokio::join!(task1);

    println!("WebSocket connection closed.");
    
}

