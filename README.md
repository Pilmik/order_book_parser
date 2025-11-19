# Order book parser

This project implements a parser for order book data, which is commonly used in financal markets to represet buy (bids) and sell (asks) orders for a security. The parser uses the Pest grammar to parse a string representation of an order book in the format "BIDS:level_list;ASKS:level_list". 

The parsing results can be used in trading applications, market analysis tools, or simulations to process and analyze order book snapshots. In particular, this parser includes a Matching Engine that allows you to simulate Buy/Sell Market orders and predict the PnL (Profit and Loss) of open positions based on the actual market depth.

---

## Current status
The parser offers the following capabilities:
* Grammar-Based Parsing: Safely processes string snapshots using PEG.
* Strict Financial Validation:
    * Validates Tick Size (price granularity).
    * Validates Minimum Lot and Lot Step (quantity granularity).
    * Ensures logical data integrity (Bids descending, Asks ascending, no Crossed Book).
* Trade Simulation:
    * Executes Market Orders with Partial Fill (IOC) logic.
    * Calculates VWAP (Volume Weighted Average Price) for entry positions.
    * Estimates Floating PnL based on remaining liquidity.

---

## Parsing process

The parser processes a string input representing an order book snapshot. The input format is:

- "BIDS:price1,quantity1|price2,quantity2|...;ASKS:price1,quantity1|price2,quantity2|..."

The Pest grammar (`grammar.pest`) defines rules for:
- Identifying "BIDS" and "ASKS" sections.
- Parsing levels as "number,number" where numbers can be integers or decimals.
- Handling whitespaces.

The parsing process transforms a raw string into a structured financial object through several stages:

1.  **Tokenization:** The Pest parser breaks the input string into tokens based on defined rules (`bids_side`, `asks_side`, `level`).
2.  **AST Generation:** An Abstract Syntax Tree is generated to represent the hierarchical structure.
3.  **Struct Conversion:** The AST is traversed to populate the `OrderBook` struct using `rust_decimal` for high-precision arithmetic (avoiding floating-point errors).
4.  **Validation Layer:** The data is validated against the user-provided `InstrumentConfig`.
5.  **Matching Engine:** If a trade is requested, the engine mutates the order book state (consumes liquidity) and returns a `Position` object.

---

## Grammar description
The parsing logic is defined in grammar.pest using PEG rules. The parser processes the input format: "BIDS:price,qty|price,qty;ASKS:price,qty|price,qty"
```pest
WHITESPACE = _{ " " | "\t" | "\r" | "\n" }

ASCII_DIGIT = {'0'..'9'}

bids_identifier = { "BIDS" }
asks_identifier = { "ASKS" }

integer = @{ ASCII_DIGIT+ }
number = @{ integer ~ ("." ~ integer)? }

// A single price level: "100.5,10"
level = { number ~ "," ~ number }

// A list of levels separated by "|": "100.5,10|100.0,5"
level_list = { (level)? ~ ("|" ~ level)* }

bids_side = { bids_identifier ~ ":" ~ level_list }
asks_side = { asks_identifier ~ ":" ~ level_list }

// Root rule
order_book = { bids_side ~ ";" ~ asks_side }
```


## CLI Usage
The project includes a CLI built with clap. To ensure data integrity, instrument configuration arguments are mandatory for parsing.

1. General info
View project information and credits:
```bash
cargo run -- credits
```

2. Parse & Validate Only
To view and validate an order book snapshot without trading (requires defining the instrument settings):
```bash
cargo run -- parse --file data/sample.txt --tick-size 0.5 --min-lot 1.0 --lot-step 1.0
```

3. Parse & Execute Trade
To parse the book AND simulate a Market Buy order for 5.0 units:
```bash
cargo run -- parse --file data/sample.txt --tick-size 0.5 --min-lot 1.0 --lot-step 1.0 --action buy --amount 5.0
```

### Output Example
The template is located in data/sample.txt folder:
```
BIDS:100.0,10|99.5,20;ASKS:101.0,5|102.0,10

```
Our command:
```bash
cargo run -- parse --file data/sample.txt --tick-size 0.5 --min-lot 1.0 --lot-step 1.0 --action buy --amount 3.0
```

Result:
```bash
Reading file: "data/sample.txt"
Applying Config: Tick=0.5, MinLot=1, Step=1

✅ Successfully parsed and validated Order Book!
Order Book:
  ASKS (Top): [Level { price: 101.0, quantity: 5 }, Level { price: 102.0, quantity: 10 }]
  BIDS (Top): [Level { price: 100.0, quantity: 10 }, Level { price: 99.5, quantity: 20 }]


--- Executing Buy Market Order for 3 ---
Result: Order Filled!
  - Quantity:    3
  - Open price:  101.0
  - PnL:    -3.0

Updated Order Book State:
Order Book:
  ASKS (Top): [Level { price: 101.0, quantity: 2 }, Level { price: 102.0, quantity: 10 }]
  BIDS (Top): [Level { price: 100.0, quantity: 10 }, Level { price: 99.5, quantity: 20 }]
```

---

## Automation
The project uses a makefile to automate routine tasks:
```bash
# Run full check cycle (Format + Lint + Test) - DO THIS BEFORE COMMIT
make check

# Run unit & integration tests
make test

# Display Help
make run
```

---

## License
License: MIT. This project was developed as part of a coursework requirement
Author: Mykhailo Pilat



