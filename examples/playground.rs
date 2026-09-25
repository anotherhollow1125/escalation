use anyhow::anyhow;
use clap::Parser;
use escalation::{RecognizedAs, Report};
use hooq::hooq;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("HogeError")]
struct HogeError;

impl RecognizedAs<HogeError> for anyhow::Error {
    fn recognized_as(&self) -> HogeError {
        HogeError
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

impl RecognizedAs<FugaError> for HogeError {
    fn recognized_as(&self) -> FugaError {
        FugaError
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

impl RecognizedAs<BarError> for FugaError {
    fn recognized_as(&self) -> BarError {
        BarError
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
