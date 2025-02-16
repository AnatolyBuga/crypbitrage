use clap::Parser;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

use crypbitrage::{
    create_unbound_channel, exchange_ws_connection, run_simple_arbitrage, CrossExchangeLOB,
    ExchangeProducerMap, LobLeg,
};

/// Helps parsing cli args.
/// Part of the binary, since it has nothing to do with the library
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// OKX INSTRUMENT NAME
    #[arg(
        short,
        long,
        value_name = "OKX_INST",
        default_value = "BTC-USD-251226-100000-C"
    )]
    okx_inst: String,

    /// DERIBIT INSTRUMENT NAME
    #[arg(
        short,
        long,
        value_name = "DERIBIT_INST",
        default_value = "BTC-26DEC25-100000-C"
    )]
    deribit_inst: String,
}

const OKX_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";
const DERIBIT_URL: &str = "wss://www.deribit.com/ws/api/v2";

// Raw Deserialised data from Exchanges
// Unofrtunately each exchange has a unique data format

/// OKX Exchange Subscription messages
#[derive(Serialize, Deserialize, Debug)]
struct OkxData {
    data: [BidsAsksOkx; 1],
}

/// Deribit Exchange Subscription messages
#[derive(Serialize, Deserialize, Debug)]
struct DeribitData {
    params: DeribitDataParams,
}

#[derive(Serialize, Deserialize, Debug)]
struct BidsAsksDeribit {
    pub asks: Vec<[f64; 2]>,
    pub bids: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BidsAsksOkx {
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    pub asks: Vec<[f64; 2]>,
    #[serde(deserialize_with = "deserialize_vec_of_arrays")]
    pub bids: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeribitDataParams {
    data: BidsAsksDeribit,
}

// Custom deserializer function
fn deserialize_vec_of_arrays<'de, D>(deserializer: D) -> Result<Vec<[f64; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Vec<Vec<String>> = Deserialize::deserialize(deserializer)?;

    raw.into_iter()
        .map(|arr| {
            let parsed: Result<Vec<f64>, _> =
                arr.iter().take(2).map(|s| s.parse::<f64>()).collect();
            parsed
                .map_err(serde::de::Error::custom)?
                .try_into()
                .map_err(|_| serde::de::Error::custom("Failed to convert to [f64; 2]"))
        })
        .collect()
}

/// 0 represents OKX Exchange Id
impl From<OkxData> for CrossExchangeLOB {
    fn from(ex_data: OkxData) -> Self {
        // known length
        let [data] = ex_data.data;
        let asks = data.asks;
        let bids = data.bids;

        let asks: LobLeg = asks.into_iter().map(|[p, q]| ((p.into(), 0), q)).collect();

        let bids: LobLeg = bids.into_iter().map(|[p, q]| ((p.into(), 0), q)).collect();

        Self { asks, bids }
    }
}

/// 1 represents Deribit Exchange Id
impl From<DeribitData> for CrossExchangeLOB {
    fn from(ex_data: DeribitData) -> Self {
        // known length
        let asks = ex_data.params.data.asks;
        let bids = ex_data.params.data.bids;

        let asks: LobLeg = asks.into_iter().map(|[p, q]| ((p.into(), 1), q)).collect();

        let bids: LobLeg = bids.into_iter().map(|[p, q]| ((p.into(), 1), q)).collect();

        Self { asks, bids }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ExchangeData {
    Okx(OkxData),
    Deribit(DeribitData),
}

impl From<ExchangeData> for CrossExchangeLOB {
    fn from(ex_data: ExchangeData) -> Self {
        match ex_data {
            ExchangeData::Okx(data) => data.into(),
            ExchangeData::Deribit(data) => data.into(),
        }
    }
}

// ### ### ###
// ### ### ###
// ### ### ###

#[tokio::main]
async fn main() {
    let cli = CliArgs::parse();

    // Channel for sending Exchange LOB updates
    // TODO Not sure about channel size
    let (sender, receiver) = create_unbound_channel::<ExchangeData>();

    // TODO Channel for sending back arbitrage exploit order

    // OKX
    let subscribe_msg_okx = json!({
        "op": "subscribe",
        "args": [
            {"channel": "books", "instId": cli.okx_inst}
        ]
    })
    .to_string();

    let (task1, producer1) = exchange_ws_connection(
        sender.clone(),
        "OKX".to_string(),
        subscribe_msg_okx,
        OKX_URL.to_string(),
    )
    .await; // Stop here till this future is complete (doesn't mean tokio::spawn is complete)

    // Deribit
    let deribit_channel = format!("book.{}.none.20.100ms", cli.deribit_inst);

    let subscribe_msg_deribit = json!({
        "method": "public/subscribe",
        "params": {"channels": [deribit_channel]},
        "jsonrpc": "2.0",
        "id": 0})
    .to_string();

    let (task2, producer2) = exchange_ws_connection(
        sender.clone(),
        "DERIBIT".to_string(),
        subscribe_msg_deribit,
        DERIBIT_URL.to_string(),
    )
    .await;

    let exchange_producer_map = ExchangeProducerMap::from_iter([(0, producer1), (1, producer2)]);

    let task3 = tokio::spawn(run_simple_arbitrage(receiver, exchange_producer_map));

    // https://stackoverflow.com/questions/69638710/when-should-you-use-tokiojoin-over-tokiospawn
    let _ = tokio::join!(task1, task2, task3);

    println!("WebSocket connection closed.");
}
