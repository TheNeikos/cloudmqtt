use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use miette::IntoDiagnostic;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::report::{Report, ReportResult};

async fn open_connection_with(path: &Path) -> miette::Result<Command> {
    let mut command = Command::new(path);

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    Ok(command)
}

pub async fn create_client_report(
    client_exe_path: PathBuf,
    parallelism: std::num::NonZeroUsize,
) -> miette::Result<Vec<Report>> {
    use futures::stream::StreamExt;

    let reports = vec![check_invalid_utf8_is_rejected(&client_exe_path)];

    futures::stream::iter(reports)
        .buffered(parallelism.get())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
}

async fn check_invalid_utf8_is_rejected(client_exe_path: &Path) -> miette::Result<Report> {
    let mut client = open_connection_with(&client_exe_path)
        .await?
        .spawn()
        .into_diagnostic()?;

    let mut to_client = client.stdin.take().unwrap();
    let mut from_client = client.stdout.take().unwrap();

    tokio::spawn(async move { from_client.read_to_end(&mut vec![]).await });

    let connack = &[
        0b0010_0000, // CONNACK
        0b0000_0010, // Remaining length
        0b0000_0000, // No session present
        0b0000_0000, // Connection accepted
    ];

    to_client.write_all(connack).await.into_diagnostic()?;

    let invalid_publish = &[
        0b0011_0000, // PUBLISH packet, DUP = 0, QoS = 0, Retain = 0
        0b0000_0111, // Length
        // Now the variable header
        0b0000_0000,
        0b0000_0010,
        0x61,
        0xC1,        // An invalid UTF-8 byte
        0b0000_0000, // Packet identifier
        0b0000_0001,
        0x1, // Payload
    ];

    to_client
        .write_all(invalid_publish)
        .await
        .into_diagnostic()?;

    let (result, output) =
        match tokio::time::timeout(Duration::from_millis(100), client.wait_with_output()).await {
            Ok(Ok(out)) => {
                if out.status.success() {
                    (ReportResult::Failure, Some(out.stderr))
                } else {
                    (ReportResult::Success, Some(out.stderr))
                }
            }
            Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
        };

    Ok(Report {
        name: String::from("Check if invalid UTF-8 is rejected"),
        description: String::from("Invalid UTF-8 is not allowed per the MQTT spec.\
                                  Any receiver should immediately close the connection upon receiving such a packet."),
        normative_statement_number: String::from("[MQTT-1.5.3-1, MQTT-1.5.3-2]"),
        result,
        output: output,
    })
}
