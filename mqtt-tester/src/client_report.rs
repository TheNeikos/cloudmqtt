use std::path::{Path, PathBuf};
use std::process::Stdio;

use miette::IntoDiagnostic;
use tokio::process::Command;

use crate::report::{Report, ReportResult};

async fn open_connection_with(path: &Path) -> miette::Result<Command> {
    let mut command = Command::new(path);

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    Ok(command)
}

pub async fn create_client_report(client_exe_path: PathBuf) -> miette::Result<Vec<Report>> {
    Ok(vec![Report {
        name: String::from("Check if invalid UTF-8 is rejected"),
        description: String::from("Invalid UTF-8 is not allowed per the MQTT spec.\
                                  Any receiver should immediately close the connection upon receiving such a packet."),
        normative_statement_number: String::from("[MQTT-1.5.3-1, MQTT-1.5.3-2]"),
        result: {
            let mut client = open_connection_with(&client_exe_path).await?.spawn().into_diagnostic()?;

            let to_client = client.stdin.take().unwrap();
            let from_client = client.stdout.take().unwrap();


            ReportResult::Success
        },
    }])
}
