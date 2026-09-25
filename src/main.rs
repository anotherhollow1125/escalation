#![feature(try_as_dyn)]

use anyhow::anyhow;
use escalation::Report;
use hooq::hooq;
use thiserror::Error;

#[hooq]
#[hooq::error_type = anyhow::Error]
#[hooq::error_kind = anyhow!("default")]
fn hoge(n: usize) -> Result<(), Report<anyhow::Error>> {
    if n == 4 {
        return Err(anyhow!("hoge error"));
    }

    Ok(())
}

#[derive(Debug, Error)]
#[error("FugaError")]
struct FugaError;

#[hooq]
#[hooq::error_type = FugaError]
#[hooq::error_kind = FugaError]
fn fuga(n: usize) -> Result<(), Report<FugaError>> {
    hoge(n)?;

    Ok(())
}

#[derive(Debug, Error)]
#[error("BarError")]
struct BarError;

#[hooq]
#[hooq::error_type = BarError]
#[hooq::error_kind = BarError]
fn bar(n: usize) -> Result<(), Report<anyhow::Error>> {
    todo!()
}

fn main() {}
