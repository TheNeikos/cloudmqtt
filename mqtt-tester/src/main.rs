//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod client_report;
mod command;
mod executable;
mod flow;
mod report;

use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};
use client_report::create_client_report;
use miette::IntoDiagnostic;
use report::{print_report, ReportResult};

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
            let reports = create_client_report(executable, args.parallelism).await?;

            let mut stdout = std::io::stdout().lock();
            for report in &reports {
                print_report(report, &mut stdout).into_diagnostic()?;
            }

            if reports.iter().any(|r| r.result != ReportResult::Success) {
                struct ReportSummary {
                    successes: usize,
                    failures: usize,
                    inconclusive: usize,
                }

                let summary = reports.iter().fold(
                    ReportSummary {
                        successes: 0,
                        failures: 0,
                        inconclusive: 0,
                    },
                    |mut sum, rep| {
                        match rep.result {
                            ReportResult::Success => sum.successes += 1,
                            ReportResult::Failure => sum.failures += 1,
                            ReportResult::Inconclusive => sum.inconclusive += 1,
                        }

                        sum
                    },
                );

                println!();
                println!(
                    "{} tests total, {} success, {} failures, {} inconclusive",
                    reports.len(),
                    summary.successes,
                    summary.failures,
                    summary.inconclusive
                );
                exit(1);
            }
        }
    }

    Ok(())
}
