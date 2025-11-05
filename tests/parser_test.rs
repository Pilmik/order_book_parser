use order_book_parser::{OrderBookParser, Rule, parse_order_book};
use pest::Parser;

#[test]
fn order_book_rule_accepts_valid_input() {
    let input = "BIDS:10.5,100|11,200;ASKS:12.0,150";
    let parsed = OrderBookParser::parse(Rule::order_book, input)
        .expect("Pest parsing failed");
    assert_eq!(parsed.count(), 1);
    let result = parse_order_book(input);
    assert!(result.is_ok());
}