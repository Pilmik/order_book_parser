use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct OrderBookParser;

pub fn parse_order_book(_input: &str) -> Result<(), anyhow::Error> {
    Ok(())
}