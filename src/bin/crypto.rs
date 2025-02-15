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

struct BidsAsks{
    pub asks: Vec<[f64;2]>,
    pub bids: Vec<[f64;2]>
}

impl From<OkxData> for BidsAsks {
    fn from(ex_data: OkxData) -> Self {
        // known length
        let [data] = ex_data.data; 
        Self { asks: data.asks, bids: data.bids }
    }
}

impl From<DeribitData> for BidsAsks {
    fn from(ex_data: DeribitData) -> Self {
        Self { asks: ex_data.params.data.asks, bids: ex_data.params.data.bids }
    }
}

// Raw DeSerialised data from Exchanges

/// OKX Exchange Subscription messages
#[derive(Serialize, Deserialize, Debug)]
struct OkxData{
    data: [BidsAsksOkx;1]
}

/// Deribit Exchange Subscription messages
#[derive(Serialize, Deserialize, Debug)]
struct DeribitData{
    params: DeribitDataParams
}

// ### ### ###

#[derive(Serialize, Deserialize, Debug)]
struct BidsAsksDeribit{
    pub asks: Vec<[f64;2]>,
    pub bids: Vec<[f64;2]>
}

#[derive(Serialize, Deserialize, Debug)]
struct BidsAsksOkx{
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    pub asks: Vec<[f64;2]>,
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    pub bids: Vec<[f64;2]>
}


#[derive(Serialize, Deserialize, Debug)]
struct DeribitDataParams{
    data: BidsAsksDeribit
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
    Deribit(DeribitData),
}

type Price = f64;
type Quantity = f64;
type ExchangeId = u8;
type IsBid = bool; // if not bid, then Ask

struct MarketOrder(Price, Quantity, IsBid, ExchangeId);

/// Simple arbitrage strategy
struct SimpleArbitrage{
    /// Combined Limit Order Book of two exchanges
    combined_lob: 
}

impl SimpleArbitrage {

}

async fn arbitrage(mut receiver: UnboundedReceiver<ExchangeData>){
    while let Some(msg) = receiver.recv().await {

        // Assume arbitrage check is a heavy CPU task 
        tokio::task::spawn_blocking( move || {
            println!("{msg:#?}")
        })
        .await
        .expect("Panic on arbitrage calc")
    }
}

#[tokio::main]
async fn main() {

    let cli = CliArgs::parse();

    // TODO Not sure about channel size
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let s1 = sender.clone();
    let s2 = sender.clone();
    
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

    let task1 = tokio::spawn( async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("OKX: {}", text);
                    // TODO Optimisation decode only data , not the whole message
                    match serde_json::from_slice::<OkxData>(text.as_bytes()) {
                        Ok(okx_data) => s1.send(ExchangeData::OKX(okx_data)).expect("OKX couldn't send"),//println!("OKX Deser: {okx_data:#?}"),
                        Err(e) => println!("OKX: couldn't deserialise: {e}")
                    } 
                }
                Ok(_) => (),
                Err(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    });




    // DERIBIT_URL
    let deribit_channel = format!("book.{}.none.20.100ms", cli.deribit_inst);

    let subscribe_msg2 = json!({
        "method": "public/subscribe",
        "params": {"channels": [deribit_channel]},
        "jsonrpc": "2.0",
        "id": 0}).to_string();

    let (ws_stream2, _) = connect_async(DERIBIT_URL).await.expect("WebSocket connection failed");
    let (mut write2, mut read2) = ws_stream2.split();

    // Send subscription request
    write2.send(Message::Text(subscribe_msg2.into()))
        .await
        .expect("Failed to send subscription message");

    println!("Subscribed to DERIBIT");

    let task2 = tokio::spawn( async move {
        while let Some(msg) = read2.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("DERIBIT: {}", text);
                    // TODO Optimisation decode only data , not the whole message
                    match serde_json::from_slice::<DeribitData>(text.as_bytes()) {
                        Ok(data) => s2.send(ExchangeData::Deribit(data)).expect("Deribit couldn't send"),
                        Err(e) => println!("Deribit: couldn't deserialise: {e}")
                    }
                }
                Ok(_) => (),
                Err(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    });

    // let task3 = tokio::task::spawn_blocking( || {arbitrage(receiver)} );

    let task3 = tokio::spawn(arbitrage(receiver));

    // https://stackoverflow.com/questions/69638710/when-should-you-use-tokiojoin-over-tokiospawn
    tokio::join!(task1, task2, task3);

    println!("WebSocket connection closed.");
    
}

