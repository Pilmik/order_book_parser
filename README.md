# Order book parser

This project implements a parser for order book data, which is commonly used in financal markets to represet buy (bids) and sell (asks) orders for a security. The parser uses the Pest grammar to parse a string representation of an order book in the format "BIDS:level_list;ASKS:level_list". The parsing results can be used in trading applications, market analysis tools, or simulations to process and analyze order book snapshots.

## Parsing process

The parser processes a string input representing an order book snapshot. The input format is:

- "BIDS:price1,quantity1|price2,quantity2|...;ASKS:price1,quantity1|price2,quantity2|..."

The Pest grammar (`grammar.pest`) defines rules for:
- Identifying "BIDS" and "ASKS" sections.
- Parsing levels as "number,number" where numbers can be integers or decimals.
- Handling whitespaces.
