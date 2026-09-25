use anyhow::anyhow;
use clap::Parser;
use escalation::{ErrorInfo, Recognize, Report};
use hooq::hooq;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("HogeError")]
struct HogeError;

impl Recognize<anyhow::Error> for HogeError {
    fn recognize(_error: &anyhow::Error) -> (Vec<ErrorInfo>, HogeError) {
        (Vec::new(), HogeError)
    }
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

impl Recognize<HogeError> for FugaError {
    fn recognize(_error: &HogeError) -> (Vec<ErrorInfo>, FugaError) {
        (Vec::new(), FugaError)
    }
}

#[hooq]
fn fuga(n: usize) -> Result<(), Report<FugaError>> {
    hoge(n)?;

    Ok(())
}

#[derive(Debug, Error)]
#[error("BarError")]
struct BarError;

impl Recognize<FugaError> for BarError {
    fn recognize(_error: &FugaError) -> (Vec<ErrorInfo>, BarError) {
        (Vec::new(), BarError)
    }
}

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
