use anyhow::anyhow;
use clap::Parser;
use escalation::{Classify, Handleable, Report, Unclassified, classify};
use hooq::hooq;
use thiserror::Error;

#[derive(Debug, Error, Classify)]
#[error("HogeError")]
struct HogeError;

classify! {
    anyhow::Error => HogeError;
    HogeError => FugaError;
    FugaError => BarError::Other;
}

#[hooq(escalate)]
fn hoge(n: usize) -> Result<(), Report<HogeError>> {
    if n.is_multiple_of(4) {
        return Err(anyhow!("hoge error"));
    }

    Ok(())
}

#[derive(Debug, Error, Classify)]
#[error("FugaError")]
struct FugaError;

#[hooq(escalate)]
#[hooq::error = FugaError]
fn fuga(n: usize) -> Result<(), Report<FugaError>> {
    let res = hoge(n);

    #[hooq::skip]
    match res.handle() {
        Ok(()) => Ok(()),
        Err((e, cause, trace)) => {
            eprintln!("[in fuga] {cause:?} {trace:?}");

            Err(Report::from_parts(e, cause, trace))
        }
    }
}

#[derive(Debug, Error, Classify)]
#[classify(Unclassified => Self::Other)]
enum BarError {
    #[error("just 4")]
    JustFour,
    #[error("other")]
    Other,
}

#[hooq(escalate)]
fn bar(n: usize) -> Result<(), Report<BarError>> {
    if n == 4 {
        #[hooq::error = BarError::JustFour]
        fuga(n)?;
    } else {
        let r = fuga(n);

        if let Err(report) = r {
            eprintln!(
                "[in bar] {:?} {:?} {:?}",
                report.peek(),
                report.cause(),
                report.trace()
            );

            return Err(report);
        }
    }

    if n > 1000 {
        return Err(anyhow::anyhow!("Unclassified")).map_err(Unclassified::from);
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
