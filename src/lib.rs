use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use futures_util::{SinkExt, StreamExt};
use ordered_float::OrderedFloat;
use serde::de::DeserializeOwned;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// OrderedFloat is needed to use Price in BTreeMap
pub type Price = OrderedFloat<f64>;
pub type Quantity = f64;
pub type ExchangeId = u8;
/// Bids or Asks
pub type LobLeg = BTreeMap<(Price, ExchangeId), Quantity>;
/// Message to Execute this Order on the exchnage
pub type OrderExecMessage = Message;

/// Maps Exchange Id to a producer (which is used to send message to that particular exchange)
pub type ExchangeProducerMap = HashMap<u8, UnboundedSender<Message>>;

#[derive(Default, Debug)]
pub struct CrossExchangeLOB {
    pub asks: LobLeg,
    pub bids: LobLeg,
}

impl CrossExchangeLOB {
    pub fn update(&mut self, other: CrossExchangeLOB) {
        self.asks.extend(other.asks);
        self.bids.extend(other.bids);
    }
}

/// Main Function for computing Simple Arbitrage
/// Builds In Memory Cross Exchange LOB from updates received by receiver
/// Evaluates if Simple Arbitrage is present
/// Sends orders on how to execute it
pub async fn run_simple_arbitrage<T: Into<CrossExchangeLOB> + Send + 'static>(
    mut receiver: UnboundedReceiver<T>,
    _exchange_producer_map: ExchangeProducerMap,
) {
    let _lob = Arc::new(Mutex::new(CrossExchangeLOB::default()));

    while let Some(msg) = receiver.recv().await {
        let lob = Arc::clone(&_lob);

        // Assume arbitrage check is a heavy CPU task
        tokio::task::spawn_blocking(move || {
            let update_data: CrossExchangeLOB = msg.into();
            lob.lock().unwrap().update(update_data);
            println!("{lob:#?}")

            // TODO
            // check if max bid is less than min ask
            // eg if bid is 101$ on exch 0 and ask is 100$ on exch 1
            // then buy on exch 1 @ 100$, sell on exch 0 for 101$
            // producer can be retrieved via the _exchange_producer_map
        })
        .await
        .expect("Panic on arbitrage calc")
    }
}

/// Create channel
pub fn create_unbound_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    tokio::sync::mpsc::unbounded_channel()
}

/// sender - sends exchange data to downstream consumers (Arbitrage Strategies)
pub async fn exchange_ws_connection<T>(
    sender: UnboundedSender<T>,
    exchange_name: String,
    subscribe_msg: String,
    url: String,
    // receiver: receives data from strategies to be sent to exchange -per exchange
) -> (JoinHandle<()>, UnboundedSender<OrderExecMessage>)
where
    T: Send + DeserializeOwned + 'static,
{
    // Open a MPSC channel here (to be used as SPSC)
    // receiver stays with with exchange and listens
    // producer is returned and to be used in downstream arb strategy
    let (order_sender, mut order_receiver) = create_unbound_channel::<OrderExecMessage>();

    // Start websocket sesh
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("WebSocket connection failed");
    let (mut write, mut ws_reader) = ws_stream.split();

    // Send subscription request
    write
        .send(Message::Text(subscribe_msg.into()))
        .await
        .unwrap_or_else(|e| {
            panic!(
                "{} Failed to send subscription message: {}",
                exchange_name, e
            )
        });

    println!("Subscribed to {}", exchange_name);

    let exch_name_clone = exchange_name.clone();

    // Listen to write instructions
    tokio::spawn(async move {
        while let Some(msg) = order_receiver.recv().await {
            write.send(msg).await.unwrap_or_else(|e| {
                panic!(
                    "{} Failed to send order exec message: {}",
                    exch_name_clone, e
                )
            });
        }
    });

    // Listen to Exchange data coming in
    (
        tokio::spawn(async move {
            while let Some(msg) = ws_reader.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        println!("{}: {}", exchange_name, text);
                        // TODO Optimisation decode only data , not the whole message
                        match serde_json::from_slice::<T>(text.as_bytes()) {
                            Ok(exchange_data) => sender.send(exchange_data).unwrap_or_else(|e| {
                                panic!(
                                    "{} data deserialized, but couldn't send: {}",
                                    exchange_name, e
                                )
                            }),
                            Err(e) => println!("{}: couldn't deserialize: {e}", exchange_name),
                        }
                    }
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{} WebSocket error: {:?}", exchange_name, e);
                        break;
                    }
                }
            }
        }),
        order_sender,
    )
}
