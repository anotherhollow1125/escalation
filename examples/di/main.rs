use clap::Parser;
use escalation::Handleable;
use std::sync::Arc;

use crate::feature::{port::FeatureRepository, usecase::Usecase, usecase::impls::UsecaseImpl};

mod db;
mod feature;

#[derive(Debug, Parser)]
struct Cli {
    n: String,
}

fn main() {
    let Cli { n } = Cli::parse();

    let repository: Arc<dyn FeatureRepository> = Arc::new(db::Db);
    let usecase = UsecaseImpl { repository };

    let res = usecase.usecase(n);

    match res.handle() {
        Ok(()) => println!("Ok"),
        Err((e, report)) => {
            eprintln!("Error: {e}\ntrace:");

            for error_info in report.into_iter().rev() {
                eprintln!("[{error_info}] {}", error_info.expr);
            }
        }
    }
}
