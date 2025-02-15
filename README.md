# Structure explained
The idea of the binary (not the lib) to 1)connect to and 2)receive data from **two** exchanges. Then 3) Assert if there is arbitrage and 4) send the required messages back to the exchange to exploit arbitrage.

Currently we only provide incomplete functionality to exploit Simple Arbitrage (which we defined as: _there is an ask on one exchange lower than a bid in another exchange_ - then we buy @ ask price (eg $98) and sell @ bid price (eg $100)).

The library provide utils to extend this set up to as many exchanges as required, and be easily extendable to other arbitrage strategies (eg Call Put parity, Calendar etc).

In other words, exchange URLs, inst names for subscription, data serializes - are unique for each exchange and must be provided on case by case basis.

Based on the above, this is what is required to set up the program to explo
Each message from exchange must be covertable into `CrossExchangeLOB`.

Binary hardcodes
It all starts from data from exhanges. 
- Each has a unique data set.


# Limitations, TODOs
All `TODOs` are also marked in the code.
- Limited to 1 instrument and two exchanges => expand.
- Ignoring Sequence and timestamps - for now assume messages come in the "correct" order
- Ser/Deser could be much more efficient
- Assuming OKX data (in response, which is a list) is always of len 1

# Questions
- OKX Sends action `snapshot`/`update`. Assuming Update **to a Price level** is the new total Quantity (from that it follows that update to quantity 0 is a cancel(or match) of all orders at that price).
