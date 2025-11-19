use pest::Parser;
use pest_derive::Parser;
use rust_decimal::Decimal;
use std::fmt;
use thiserror::Error;

/// The main parser structure for processing Order Book snapshots.
///
/// This struct uses the Pest grammar defined in `grammar.pest` to parse string inputs.
///
/// # Grammar Rules Description
/// The parser relies on the following PEG (Parsing Expression Grammar) rules:
///
/// - **`WHITESPACE`**: Handles silent whitespace characters (spaces, tabs, newlines).
///   Defined as: `_{ " " | "\t" | "\r" | "\n" }`
///
/// - **`ASCII_DIGIT`**: Matches any single digit from '0' to '9'.
///
/// - **`integer`**: Matches a sequence of one or more digits.
///   Defined as: `@{ ASCII_DIGIT+ }`
///
/// - **`number`**: Matches a financial number, which can be an integer or a decimal.
///   Defined as: `@{ integer ~ ("." ~ integer)? }`
///
/// - **`level`**: Represents a single price level consisting of a Price and a Quantity.
///   Format: `price,quantity` (e.g., "100.5,10").
///   Defined as: `{ number ~ "," ~ number }`
///
/// - **`level_list`**: Represents a sequence of levels separated by a pipe `|`.
///   Defined as: `{ (level)? ~ ("|" ~ level)* }`
///
/// - **`bids_side`**: Identifies the Buy side of the order book.
///   Format: `BIDS:level_list`
///
/// - **`asks_side`**: Identifies the Sell side of the order book.
///   Format: `ASKS:level_list`
///
/// - **`order_book`**: The root rule that combines both sides.
///   Format: `bids_side ~ ";" ~ asks_side`
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct OrderBookParser;

/// Custom error type for the Order Book library.
#[derive(Error, Debug)]
pub enum OrderBookError {
    /// Error propagated from the Pest parser (grammar mismatch).
    #[error("Failed to parse the input string: {0}")]
    ParseError(Box<pest::error::Error<Rule>>),

    /// Error parsing a string into a Decimal number.
    #[error("Failed to parse number: {0}")]
    DecimalError(#[from] rust_decimal::Error),

    /// Logical error when a required section (like Price or Quantity) is missing.
    #[error("Missing required section: {0}")]
    MissingSection(String),

    /// Validation error: Bids are not sorted descending.
    #[error("Bids must be sorted descending (highest first). Found issue at price {0}")]
    BidsUnsorted(Decimal),

    /// Validation error: Asks are not sorted ascending.
    #[error("Asks must be sorted ascending (lowest first). Found issue at price {0}")]
    AsksUnsorted(Decimal),

    /// Validation error: Duplicate price levels exist.
    #[error("Duplicate price level found: {0}")]
    DuplicatePrice(Decimal),

    /// Validation error: The best Bid is higher than or equal to the best Ask.
    #[error("Crossed book detected: Best Bid ({0}) is >= Best Ask ({1})")]
    CrossedBook(Decimal, Decimal),

    /// Instrument validation: Price is not a multiple of the tick size.
    #[error("Price {0} is not a multiple of tick size {1}")]
    InvalidTickSize(Decimal, Decimal),

    /// Instrument validation: Quantity is below the minimum lot.
    #[error("Quantity {0} is less than minimum lot size {1}")]
    InvalidMinLot(Decimal, Decimal),

    /// Instrument validation: Quantity is not a multiple of the lot step.
    #[error("Quantity {0} is not a multiple of lot step {1}")]
    InvalidLotStep(Decimal, Decimal),

    /// Trading error: Not enough liquidity in the book to fill the order.
    #[error("Not enough liquidity to fill order. Requested: {0}, Available: {1}")]
    NotEnoughLiquidity(Decimal, Decimal),
}

// Implement manual From to handle the Boxed error
impl From<pest::error::Error<Rule>> for OrderBookError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        OrderBookError::ParseError(Box::new(err))
    }
}

/// Configuration for a specific financial instrument.
/// Defines rules for validation like tick size and minimum lot.
#[derive(Debug, Clone)]
pub struct InstrumentConfig {
    /// Minimum price movement (e.g., 0.01 or 0.0005).
    pub tick_size: Decimal,
    /// Minimum quantity allowed for an order.
    pub min_lot: Decimal,
    /// Step by which quantity can increase.
    pub lot_step: Decimal,
}

impl InstrumentConfig {
    /// Creates a new configuration from f64 values.
    pub fn new(tick_size: f64, min_lot: f64, lot_step: f64) -> Self {
        Self {
            tick_size: Decimal::from_f64_retain(tick_size).unwrap_or_default(),
            min_lot: Decimal::from_f64_retain(min_lot).unwrap_or_default(),
            lot_step: Decimal::from_f64_retain(lot_step).unwrap_or_default(),
        }
    }
}

/// Represents the side of a trade (Buy or Sell).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Side {
    Buy,
    Sell,
}

/// Represents a single price level in the order book (Price and Quantity).
#[derive(Debug, PartialEq, Clone)]
pub struct Level {
    pub price: Decimal,
    pub quantity: Decimal,
}

/// Represents the full Order Book containing Bids and Asks.
#[derive(Debug, Default, Clone)]
pub struct OrderBook {
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

/// Represents an open position resulting from a trade execution.
#[derive(Debug, Clone)]
pub struct Position {
    pub side: Side,
    pub quantity: Decimal,
    /// Volume Weighted Average Price of the entry.
    pub entry_price: Decimal,
}

impl Position {
    /// Calculates Unrealized PnL (Profit and Loss) based on the current Order Book state.
    ///
    /// * **Long (Buy)** positions close at the best available **Bid** price.
    /// * **Short (Sell)** positions close at the best available **Ask** price.
    ///
    /// Returns `None` if there is no liquidity to calculate the exit price.
    pub fn calculate_pnl(&self, book: &OrderBook) -> Option<Decimal> {
        match self.side {
            Side::Buy => {
                // Long: We sell at the Best Bid
                let best_bid = book.bids.first()?.price;
                Some((best_bid - self.entry_price) * self.quantity)
            }
            Side::Sell => {
                // Short: We buy back at the Best Ask
                let best_ask = book.asks.first()?.price;
                Some((self.entry_price - best_ask) * self.quantity)
            }
        }
    }
}

impl OrderBook {
    /// Executes a Market Order with Partial Fill logic (IOC).
    ///
    /// This method mutates the order book by consuming liquidity from the opposite side.
    ///
    /// # Arguments
    /// * `side` - The direction of the trade (Buy or Sell).
    /// * `quantity` - The amount to trade.
    ///
    /// # Returns
    /// * `Ok(Position)` - The resulting position with the weighted average entry price.
    /// * `Err(OrderBookError)` - If the order is invalid or book is empty.
    pub fn execute_market_order(
        &mut self,
        side: Side,
        quantity: Decimal,
    ) -> Result<Position, OrderBookError> {
        if quantity <= Decimal::ZERO {
            return Err(OrderBookError::NotEnoughLiquidity(quantity, Decimal::ZERO));
        }

        let levels = match side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };

        let mut remaining_qty = quantity;
        let mut total_cost = Decimal::ZERO;
        let mut filled_qty = Decimal::ZERO;

        let mut i = 0;
        while i < levels.len() && remaining_qty > Decimal::ZERO {
            let level = &mut levels[i];

            if level.quantity <= remaining_qty {
                let trade_qty = level.quantity;
                total_cost += level.price * trade_qty;
                filled_qty += trade_qty;
                remaining_qty -= trade_qty;
                levels.remove(i);
            } else {
                let trade_qty = remaining_qty;
                total_cost += level.price * trade_qty;
                filled_qty += trade_qty;
                level.quantity -= trade_qty;
                remaining_qty = Decimal::ZERO;
                i += 1;
            }
        }

        if filled_qty == Decimal::ZERO {
            return Err(OrderBookError::NotEnoughLiquidity(quantity, Decimal::ZERO));
        }

        let avg_price = total_cost / filled_qty;

        Ok(Position {
            side,
            quantity: filled_qty,
            entry_price: avg_price,
        })
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Order Book:")?;
        writeln!(
            f,
            "  ASKS (Top): {:?}",
            self.asks.iter().take(3).collect::<Vec<_>>()
        )?;
        writeln!(
            f,
            "  BIDS (Top): {:?}",
            self.bids.iter().take(3).collect::<Vec<_>>()
        )
    }
}

/// Parses a raw string input into an `OrderBook` struct.
///
/// Validates the structure against the grammar and business rules (Instrument Config).
///
/// # Arguments
/// * `input` - The string snapshot (e.g., "BIDS:100,1;ASKS:101,1").
/// * `config` - Optional instrument configuration for strict validation.
pub fn parse_order_book(
    input: &str,
    config: Option<&InstrumentConfig>,
) -> Result<OrderBook, OrderBookError> {
    let mut parsed = OrderBookParser::parse(Rule::order_book, input)?;
    let root = parsed
        .next()
        .ok_or_else(|| OrderBookError::MissingSection("Empty input".into()))?;

    let mut book = OrderBook::default();

    for record in root.into_inner() {
        match record.as_rule() {
            Rule::bids_side => book.bids = parse_levels(record)?,
            Rule::asks_side => book.asks = parse_levels(record)?,
            _ => {}
        }
    }

    validate_book_logic(&book)?;
    if let Some(cfg) = config {
        validate_instrument_rules(&book, cfg)?;
    }

    Ok(book)
}

fn parse_levels(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Level>, OrderBookError> {
    let mut levels = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::level_list {
            for level_pair in inner.into_inner() {
                if level_pair.as_rule() == Rule::level {
                    let mut nums = level_pair.into_inner();
                    let price_str = nums
                        .next()
                        .ok_or_else(|| OrderBookError::MissingSection("Missing price".into()))?
                        .as_str();
                    let qty_str = nums
                        .next()
                        .ok_or_else(|| OrderBookError::MissingSection("Missing quantity".into()))?
                        .as_str();
                    levels.push(Level {
                        price: Decimal::from_str_exact(price_str)?,
                        quantity: Decimal::from_str_exact(qty_str)?,
                    });
                }
            }
        }
    }
    Ok(levels)
}

fn validate_book_logic(book: &OrderBook) -> Result<(), OrderBookError> {
    for window in book.bids.windows(2) {
        if window[0].price == window[1].price {
            return Err(OrderBookError::DuplicatePrice(window[0].price));
        }
        if window[0].price < window[1].price {
            return Err(OrderBookError::BidsUnsorted(window[1].price));
        }
    }
    for window in book.asks.windows(2) {
        if window[0].price == window[1].price {
            return Err(OrderBookError::DuplicatePrice(window[0].price));
        }
        if window[0].price > window[1].price {
            return Err(OrderBookError::AsksUnsorted(window[1].price));
        }
    }
    if let (Some(bid), Some(ask)) = (book.bids.first(), book.asks.first())
        && bid.price >= ask.price
    {
        return Err(OrderBookError::CrossedBook(bid.price, ask.price));
    }
    Ok(())
}

fn validate_instrument_rules(
    book: &OrderBook,
    config: &InstrumentConfig,
) -> Result<(), OrderBookError> {
    let all_levels = book.bids.iter().chain(book.asks.iter());
    for level in all_levels {
        if !(level.price % config.tick_size).is_zero() {
            return Err(OrderBookError::InvalidTickSize(
                level.price,
                config.tick_size,
            ));
        }
        if level.quantity < config.min_lot {
            return Err(OrderBookError::InvalidMinLot(
                level.quantity,
                config.min_lot,
            ));
        }
        if !(level.quantity % config.lot_step).is_zero() {
            return Err(OrderBookError::InvalidLotStep(
                level.quantity,
                config.lot_step,
            ));
        }
    }
    Ok(())
}
