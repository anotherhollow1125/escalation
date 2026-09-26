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
    FugaError => BarError::Other;
}

#[hooq]
fn hoge(n: usize) -> Result<(), Report<HogeError>> {
    if n.is_multiple_of(4) {
        return Err(anyhow!("hoge error"));
    }

    Ok(())
}

#[derive(Debug, Error)]
#[error("FugaError")]
struct FugaError;

#[hooq]
#[hooq::error = FugaError]
fn fuga(n: usize) -> Result<(), Report<FugaError>> {
    hoge(n)?;

    Ok(())
}

#[derive(Debug, Error)]
enum BarError {
    #[error("just 4")]
    JustFour,
    #[error("other")]
    Other,
}

#[hooq]
fn bar(n: usize) -> Result<(), Report<BarError>> {
    if n == 4 {
        #[hooq::error = BarError::JustFour]
        fuga(n)?;
    } else {
        fuga(n)?;
    }

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
