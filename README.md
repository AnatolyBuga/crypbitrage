# Structure explained
The idea of the binary (and the lib):
1) connect to **two** exchanages
2) receive data from **two** exchanges. _prints it to console_
3) Send data via a channel to the strategy(arbitrage seeking) thread.
4) i) Build Cross Exchange LOB  
   ii) Assert if there is arbitrage
5) sends the required messages back to the exchange to exploit arbitrage.

Currently we only provide incomplete functionality to exploit Simple Arbitrage (which we defined as: _there is an ask on one exchange lower than a bid in another exchange_ - then we buy @ ask price (eg $98) and sell @ bid price (eg $100)).

The library provide utils to extend this set up to as many exchanges as required, and be easily extendable to other arbitrage strategies (eg Call Put parity, Calendar etc).

In other words, exchange URLs, inst names for subscription, data serializes - are unique for each exchange and must be provided on case by case basis. These are hardcoded.

<div style="color: red; font-weight: bold;">
⚠ Warning: You should provide inst names to the binary. Below is an example with default values - these can be dropped.
</div>

# How to run
`cargo run -- --okx-inst BTC-USD-251226-100000-C --deribit-inst BTC-26DEC25-100000-C`

# How to extend
Based on the above, this is what is required to set up the program to explo
Each message from exchange must be covertable into `Into<CrossExchangeLOB>`. In our case we implement `ExchangeData` enum.

Binary hardcodes
It all starts from data from exhanges. 
- Each has a unique data set.


# TODOs (in order of priority)
All `TODOs` are also marked in the code.
- Serialization is done via Serde. While this is convenient - it is not good enough for low latency app. We should write our own serialisers from bytes for max efficiency. i) We know the expected format of the data and ii) we know which data we need.
- The example (binary) is for 1 instrument and two exchanges. This can be extended.
- Ignoring Sequence and timestamps - for now assume messages come in the "correct" order
- Ser/Deser could be much more efficient
- Assuming OKX data (in response, which is a list) is always of len 1

# Questions
- OKX Sends action `snapshot`/`update`. Assuming Update **to a Price level** is the new total Quantity (from that it follows that update to quantity 0 is a cancel(or match) of all orders at that price).
