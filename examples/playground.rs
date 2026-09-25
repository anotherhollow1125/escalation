use anyhow::anyhow;
use clap::Parser;
use escalation::{Report, classify};
use hooq::hooq;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("HogeError")]
struct HogeError;

classify! {
    anyhow::Error => HogeError;
    HogeError => FugaError;
    FugaError => BarError;
}

#[hooq]
fn hoge(n: usize) -> Result<(), Report<HogeError>> {
    if n == 4 {
        return Err(anyhow!("hoge error"));
    }

    Ok(())
}

#[derive(Debug, Error)]
#[error("FugaError")]
struct FugaError;

#[hooq]
fn fuga(n: usize) -> Result<(), Report<FugaError>> {
    hoge(n)?;

    Ok(())
}

#[derive(Debug, Error)]
#[error("BarError")]
struct BarError;

#[hooq]
fn bar(n: usize) -> Result<(), Report<BarError>> {
    fuga(n)?;

    Ok(())
}

#[derive(Debug, Parser)]
struct Cli {
    n: usize,
}

fn main() {
    let Cli { n } = Cli::parse();

    if let Err(e) = bar(n) {
        dbg!(e);
    }
}
