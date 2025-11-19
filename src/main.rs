use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use order_book_parser::{InstrumentConfig, Side, parse_order_book};
use rust_decimal::Decimal;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "order_book_parser")]
#[command(author = "Mykhailo Pilat")]
#[command(version = "0.1.0")]
#[command(about = "Parses financial order book snapshots with strict validation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum TradeSide {
    Buy,
    Sell,
}

#[derive(Subcommand)]
enum Commands {
    /// Parses a file. Requires full instrument configuration.
    Parse {
        /// Path to the input file containing order book data.
        #[arg(short, long)]
        file: PathBuf,

        /// Instrument Tick Size (e.g., 0.5). REQUIRED.
        #[arg(long)]
        tick_size: f64,

        /// Instrument Minimum Lot size (e.g., 1.0). REQUIRED.
        #[arg(long)]
        min_lot: f64,

        /// Instrument Lot Step (e.g., 1.0). REQUIRED.
        #[arg(long)]
        lot_step: f64,

        /// Action to perform: 'buy' or 'sell'.
        #[arg(long, requires = "amount")]
        action: Option<TradeSide>,

        /// Amount to trade. Must match min_lot and lot_step rules.
        #[arg(long, requires = "action")]
        amount: Option<f64>,
    },
    /// Displays credits information.
    Credits,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Credits => {
            println!("========================================");
            println!("   Order Book Parser v0.1.0");
            println!("========================================");
            println!("Author:  Mykhailo Pilat");
            println!("License: MIT");
            println!("========================================");
        }
        Commands::Parse {
            file,
            tick_size,
            min_lot,
            lot_step,
            action,
            amount,
        } => {
            println!("Reading file: {:?}", file);
            let content = fs::read_to_string(&file)
                .with_context(|| format!("Could not read file `{:?}`", file))?;
            let cleaned_content = content.trim();

            println!(
                "Applying Config: Tick={}, MinLot={}, Step={}",
                tick_size, min_lot, lot_step
            );
            let config = InstrumentConfig::new(tick_size, min_lot, lot_step);

            match parse_order_book(cleaned_content, Some(&config)) {
                Ok(mut book) => {
                    println!("\n✅ Successfully parsed and validated Order Book!");
                    println!("{}", book);

                    if let (Some(trade_side), Some(trade_amount_f64)) = (action, amount) {
                        let trade_qty =
                            Decimal::from_f64_retain(trade_amount_f64).unwrap_or_default();

                        validate_order_params(trade_qty, &config)?;

                        let lib_side = match trade_side {
                            TradeSide::Buy => Side::Buy,
                            TradeSide::Sell => Side::Sell,
                        };

                        perform_trade(&mut book, lib_side, trade_qty)?;
                    }
                }
                Err(e) => {
                    eprintln!("\n Error processing order book:");
                    eprintln!("   {}", e);
                    eprintln!("   Hint: Check if your file data complies with the tick/lot rules.");
                }
            }
        }
    }

    Ok(())
}

fn validate_order_params(qty: Decimal, config: &InstrumentConfig) -> Result<()> {
    if qty < config.min_lot {
        bail!(
            "Order amount {} is less than minimum lot {}",
            qty,
            config.min_lot
        );
    }
    if !(qty % config.lot_step).is_zero() {
        bail!(
            "Order amount {} must be a multiple of lot step {}",
            qty,
            config.lot_step
        );
    }
    Ok(())
}

fn perform_trade(book: &mut order_book_parser::OrderBook, side: Side, qty: Decimal) -> Result<()> {
    println!("\n--- Executing {:?} Market Order for {} ---", side, qty);

    match book.execute_market_order(side, qty) {
        Ok(position) => {
            println!("Result: Order Filled!");
            println!("  - Quantity:    {}", position.quantity);
            println!("  - Open price:  {}", position.entry_price.round_dp(4));

            if let Some(pnl) = position.calculate_pnl(book) {
                println!("  - PnL:    {}", pnl.round_dp(2));
            } else {
                println!("  - PnL:    N/A (Insufficient liquidity to calc exit)");
            }

            println!("\nUpdated Order Book State:");
            println!("{}", book);
        }
        Err(e) => {
            eprintln!("Trade Failed: {}", e);
        }
    }
    Ok(())
}
