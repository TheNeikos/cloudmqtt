//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use futures::FutureExt;
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

    let reports = vec![
        check_invalid_utf8_is_rejected(&client_exe_path).boxed_local(),
        check_receiving_server_packet(&client_exe_path).boxed_local(),
    ];

    futures::stream::iter(reports)
        .buffered(parallelism.get())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
}

async fn check_invalid_utf8_is_rejected(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([
            crate::command::ClientCommand::Send(&[
                0b0010_0000, // CONNACK
                0b0000_0010, // Remaining length
                0b0000_0000, // No session present
                0b0000_0000, // Connection accepted
            ]),
            crate::command::ClientCommand::Send(&[
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
            ]),
        ]);

    let (result, output) = match tokio::time::timeout(Duration::from_millis(100), output).await {
        Ok(Ok(out)) => (
            if out.status.success() {
                ReportResult::Failure
            } else {
                ReportResult::Success
            },
            Some(out.stderr),
        ),
        Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
    };

    Ok(Report {
        name: String::from("Check if invalid UTF-8 is rejected"),
        description: String::from("Invalid UTF-8 is not allowed per the MQTT spec.\
                                  Any receiver should immediately close the connection upon receiving such a packet."),
        normative_statement_number: String::from("[MQTT-1.5.3-1, MQTT-1.5.3-2]"),
        result,
        output,
    })
}

async fn check_receiving_server_packet(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([
            crate::command::ClientCommand::Send(&[
                0b0010_0000, // CONNACK
                0b0000_0010, // Remaining length
                0b0000_0000, // No session present
                0b0000_0000, // Connection accepted
            ]),
            crate::command::ClientCommand::Send(&[
                0b1000_0010, // SUBSCRIBE packet
                0b0000_1000, // Length
                // Now the variable header
                0b0000_0000, // Packet ID
                0b0000_0001,
                // First sub
                0b0000_0000,
                0b0000_0011, // Length
                b'a',
                b'/',
                b'b',
                0b0000_0001, // QoS
            ]),
        ]);

    let (result, output) = match tokio::time::timeout(Duration::from_millis(100), output).await {
        Ok(Ok(out)) => (
            if out.status.success() {
                ReportResult::Failure
            } else {
                ReportResult::Success
            },
            Some(out.stderr),
        ),
        Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
    };

    Ok(Report {
        name: String::from("Check if invalid packets are rejected"),
        description: String::from(
            "Unexpected packets are a protocol error and the client MUST close the connection.",
        ),
        normative_statement_number: String::from("[MQTT-4.8.0-1]"),
        result,
        output,
    })
}
