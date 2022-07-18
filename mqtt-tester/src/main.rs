mod client_report;
mod report;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use client_report::create_client_report;

#[derive(Parser, Debug)]
#[clap(author, version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    #[clap(long, default_value = "10")]
    parallelism: std::num::NonZeroUsize,
}

#[derive(Subcommand, Debug)]
enum Commands {
    TestClient {
        #[clap(value_parser)]
        executable: PathBuf,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::TestClient { executable } => {
            let report = create_client_report(executable, args.parallelism).await?;

            println!("Report: {:#?}", report);
        }
    }

    Ok(())
}
