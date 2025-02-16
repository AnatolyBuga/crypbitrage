# <div style="color: lightblue; font-weight: bold;">Structure explained</div>
Program structure:
1) connect to **two** exchanages
2) receive data from **two** exchanges. _prints that data to console_
3) Send data via a channel to the strategy(arbitrage seeking) thread.
4) i) Build Cross Exchange LOB   _prints to console_
   ii) Assert if there is arbitrage
5) Send the required requests back to the exchanges to exploit arbitrage.

The printed Cross Exchange LOB to be read as follows:
```
asks: {
            (
                0.234, # price
                0,     # exchange Id (to identify which exchange order came from)
            ): 606.0,  # Quantity
```

Exploit **Simple Arbitrage** logic is WIP (which I defined as: _there is an ask on one exchange lower than a bid in another exchange_ - then we buy @ ask price (eg $98) and sell @ bid price (eg $100)).

The library provides utils to extend this set up to as many exchanges as required, and be easily extendable to other arbitrage strategies (eg Call Put parity, Calendar etc).

However, exchange URLs, inst names for subscription, (incoming/outgoing) data/message structures - are unique for each exchange and must be provided on case by case basis. These are hardcoded in the binary for our **two** exchanges: **OKX** and **DERIBIT**.

<div style="color: red;">
⚠ Warning: You could provide inst names to the binary. Below is an example with default values - these can be dropped.
</div>

# <div style="color: lightblue; font-weight: bold;">How to run </div>
`cargo run -- --okx-inst BTC-USD-251226-100000-C --deribit-inst BTC-26DEC25-100000-C`

# <div style="color: lightblue; font-weight: bold;">How to extend </div>

Due to exchange specific items listed in the first section - each message from exchange must be convertible into `Into<CrossExchangeLOB>`. In other words, you must (de)serialize exchange data. In this case I implement it for `ExchangeData` enum. This must be correctly reflected in `ExchangeProducerMap` (a map from exchange Id to the exchange-unique producer which sends orders to exchange).

# <div style="color: lightblue; font-weight: bold;"> TODOs </div> (in order of priority)
All `TODOs` are also marked in the code.
- Serialization is done via Serde. While this is convenient - it is not good enough for low latency app. One should write their own serializers from bytes for max efficiency. i) We know the expected format of the data and ii) we know which data we need.
- Further code restructuring - one should introduce Exchange and ExchangeHandle structs (Actor model). 
- Ignoring Sequence and timestamps - for now assume messages come in the "correct" order
- We are making some serious assumptions about data. eg. Assuming OKX data (in response, which is a list) is always of len 1

# <div style="color: lightblue; font-weight: bold;"> Questions </div>
- OKX Sends action `snapshot`/`update`. Assuming Update **to a Price level** is the new total Quantity (from that it follows that update to quantity 0 is a cancel(or match) of all orders at that price).
