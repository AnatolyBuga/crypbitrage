# Limitations, TODOs
- Limited to 1 instrument and two exchanges => expand.
- Ignoring Sequence and timestamps - for now assume messages come in the "correct" order
- Ser/Deser could be much more efficient

# Questions
- OKX Sends action `snapshot`/`update`. Assuming Update to a Price level is the new total Quantity (from that it follows that update to quantity 0 is a cancel).
