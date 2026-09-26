use clap::Parser;
use std::sync::Arc;

use crate::feature::{port::FeatureRepository, usecase::Usecase, usecase::impls::UsecaseImpl};

mod db;
mod feature;

#[derive(Debug, Parser)]
struct Cli {
    n: String,
    #[arg(short = 'j', long)]
    as_json: bool,
}

fn main() {
    let Cli { n, as_json } = Cli::parse();

    let repository: Arc<dyn FeatureRepository> = Arc::new(db::Db);
    let usecase = UsecaseImpl { repository };

    let res = usecase.usecase(n);

    match res {
        Ok(()) => println!("Ok"),
        Err(r) => {
            if as_json {
                eprintln!("{}", r.into_json());
            } else {
                eprintln!("{r}");
            }
        }
    }
}
