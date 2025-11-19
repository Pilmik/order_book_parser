use anyhow::Result;
use order_book_parser::{InstrumentConfig, OrderBookParser, Rule, Side, parse_order_book};
use pest::Parser;
use rust_decimal::prelude::*;

#[test]
fn test_grammar_rule_whitespace() {
    let input = "BIDS : 100.0 , 5 ; ASKS : 102.0 , 5";
    assert!(parse_order_book(input, None).is_ok());
}

#[test]
fn test_grammar_rule_ascii_digit() {
    assert!(OrderBookParser::parse(Rule::ASCII_DIGIT, "0").is_ok());
    assert!(OrderBookParser::parse(Rule::ASCII_DIGIT, "9").is_ok());

    let mut _pairs = OrderBookParser::parse(Rule::ASCII_DIGIT, "12");
    assert!(OrderBookParser::parse(Rule::ASCII_DIGIT, "a").is_err());
}

#[test]
fn test_grammar_rule_identifiers() {
    assert!(OrderBookParser::parse(Rule::bids_identifier, "BIDS").is_ok());
    assert!(OrderBookParser::parse(Rule::asks_identifier, "ASKS").is_ok());

    assert!(OrderBookParser::parse(Rule::bids_identifier, "bids").is_err());
    assert!(OrderBookParser::parse(Rule::bids_identifier, "asks").is_err());
}

#[test]
fn test_grammar_rule_integer() {
    assert!(OrderBookParser::parse(Rule::integer, "12345").is_ok());
    assert!(OrderBookParser::parse(Rule::integer, "0").is_ok());

    assert!(OrderBookParser::parse(Rule::integer, "a12").is_err());
}

#[test]
fn test_grammar_rule_number() {
    let valid_int = "123";
    let valid_dec = "123.456";

    assert!(OrderBookParser::parse(Rule::number, valid_int).is_ok());
    assert!(OrderBookParser::parse(Rule::number, valid_dec).is_ok());
    assert!(OrderBookParser::parse(Rule::number, "abc").is_err());
}

#[test]
fn test_grammar_rule_level() {
    assert!(OrderBookParser::parse(Rule::level, "100.5,10").is_ok());
    assert!(OrderBookParser::parse(Rule::level, "50,5").is_ok());

    assert!(OrderBookParser::parse(Rule::level, "100.5").is_err());
    assert!(OrderBookParser::parse(Rule::level, "100.5,").is_err());
    assert!(OrderBookParser::parse(Rule::level, ",10").is_err());
}

#[test]
fn test_grammar_rule_level_list() {
    assert!(OrderBookParser::parse(Rule::level_list, "100.0,10").is_ok());
    assert!(OrderBookParser::parse(Rule::level_list, "100.0,10|99.5,20").is_ok());
    assert!(OrderBookParser::parse(Rule::level_list, "").is_ok());
}

#[test]
fn test_grammar_rule_bids_side() {
    assert!(OrderBookParser::parse(Rule::bids_side, "BIDS:100.0,10|99.0,5").is_ok());
    assert!(OrderBookParser::parse(Rule::bids_side, "BIDS:").is_ok());

    assert!(OrderBookParser::parse(Rule::bids_side, "BIDS 100,1").is_err());
    assert!(OrderBookParser::parse(Rule::bids_side, "ASKS:100,1").is_err());
}

#[test]
fn test_grammar_rule_asks_side() {
    assert!(OrderBookParser::parse(Rule::asks_side, "ASKS:100.0,10").is_ok());

    assert!(OrderBookParser::parse(Rule::asks_side, "ASKS-100,1").is_err());
    assert!(OrderBookParser::parse(Rule::asks_side, "BIDS:100,1").is_err());
}

#[test]
fn test_valid_order_book() -> Result<()> {
    let input = "BIDS:100.0,10|99.5,5;ASKS:102.0,20|103.5,15";
    let book = parse_order_book(input, None)?;

    assert_eq!(book.bids.len(), 2);
    assert_eq!(book.asks.len(), 2);

    assert_eq!(book.bids[0].price.to_string(), "100.0");
    assert_eq!(book.bids[0].quantity.to_string(), "10");

    Ok(())
}

#[test]
fn test_invalid_syntax_order_book() {
    let missing_semicolon = "BIDS:100.0,10 ASKS:101.0,10";
    assert!(parse_order_book(missing_semicolon, None).is_err());

    let wrong_header = "BIDS:100.0,10;ASKA:101.0,10";
    assert!(parse_order_book(wrong_header, None).is_err());

    let garbage = "BlaBlaWlaWla";
    assert!(parse_order_book(garbage, None).is_err());
}

#[test]
fn test_crossed_book_error() {
    let input = "BIDS:105.0,10;ASKS:105.0,20";
    let result = parse_order_book(input, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Crossed book detected"));
}

#[test]
fn test_unsorted_bids() {
    let input = "BIDS:100.0,1|101.0,1;ASKS:110.0,1";
    let result = parse_order_book(input, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Bids must be sorted descending"));
}

#[test]
fn test_unsorted_asks() {
    let input = "BIDS:100.0,1;ASKS:110.0,1|109.0,1";
    let result = parse_order_book(input, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Asks must be sorted ascending"));
}

#[test]
fn test_duplicate_price() {
    let input = "BIDS:100.0,1|100.0,5;ASKS:110.0,1";
    let result = parse_order_book(input, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Duplicate price level"));
}

#[test]
fn test_instrument_validation() {
    let config = InstrumentConfig::new(0.5, 10.0, 5.0);
    let valid = "BIDS:100.5,10|100.0,15;ASKS:101.0,20";
    assert!(parse_order_book(valid, Some(&config)).is_ok());

    let bad_tick = "BIDS:100.3,10;ASKS:102.0,20";
    let result_tick = parse_order_book(bad_tick, Some(&config));
    assert!(result_tick.is_err());
    assert!(format!("{}", result_tick.unwrap_err()).contains("multiple of tick size"));

    let bad_lot = "BIDS:100.0,2;ASKS:102.0,20";
    let result_lot = parse_order_book(bad_lot, Some(&config));
    assert!(result_lot.is_err());
    assert!(format!("{}", result_lot.unwrap_err()).contains("less than minimum lot size"));

    let bad_step = "BIDS:100.0,12;ASKS:102.0,20";
    let result_step = parse_order_book(bad_step, Some(&config));
    assert!(result_step.is_err());
    assert!(format!("{}", result_step.unwrap_err()).contains("multiple of lot step"));
}

#[test]
fn test_execute_market_buy_and_pnl() -> Result<()> {
    let input = "BIDS:99.0,10;ASKS:101.0,10|102.0,10";
    let mut book = parse_order_book(input, None)?;

    let position = book.execute_market_order(Side::Buy, Decimal::from(15))?;

    assert_eq!(position.side, Side::Buy);
    assert_eq!(position.quantity.to_string(), "15");

    let expected_entry = Decimal::from(1520) / Decimal::from(15);
    assert_eq!(position.entry_price, expected_entry);

    assert_eq!(book.asks.len(), 1);
    assert_eq!(book.asks[0].price.to_string(), "102.0");
    assert_eq!(book.asks[0].quantity.to_string(), "5");

    let pnl = position.calculate_pnl(&book).unwrap();

    let expected_pnl = (Decimal::from_str("99.0")? - expected_entry) * Decimal::from(15);
    assert_eq!(pnl, expected_pnl);

    Ok(())
}
#[test]
fn test_partial_fill_logic() -> Result<()> {
    let input = "BIDS:100.0,10;ASKS:101.0,5";
    let mut book = parse_order_book(input, None)?;

    let position = book.execute_market_order(Side::Buy, Decimal::from(10))?;

    assert_eq!(position.side, Side::Buy);

    assert_eq!(position.quantity.to_string(), "5");

    assert_eq!(position.entry_price.to_string(), "101.0");

    assert!(book.asks.is_empty());

    Ok(())
}
