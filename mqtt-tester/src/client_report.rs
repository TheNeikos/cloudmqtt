//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use futures::FutureExt;
use mqtt_format::v3::connect_return::MConnectReturnCode;

use mqtt_format::v3::header::MPacketKind;
use mqtt_format::v3::identifier::MPacketIdentifier;
use mqtt_format::v3::packet::{MConnack, MConnect, MPacket, MSubscribe};

use mqtt_format::v3::qos::MQualityOfService;
use mqtt_format::v3::strings::MString;
use mqtt_format::v3::subscription_request::MSubscriptionRequests;
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
        check_invalid_first_packet_is_rejected(&client_exe_path).boxed_local(),
        check_utf8_with_nullchar_is_rejected(&client_exe_path).boxed_local(),
        check_connack_flags_are_set_as_reserved(&client_exe_path).boxed_local(),
    ];

    futures::stream::iter(reports)
        .buffered(parallelism.get())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
}

macro_rules! mk_report {
    (name: $name:literal,
     desc: $description:literal,
     normative: $normative:literal,
     $result:ident,
     $output:ident) => {
        Report {
            name: String::from($name),
            description: String::from($description),
            normative_statement_number: String::from($normative),
            $result,
            $output,
        }
    };
}

async fn check_invalid_utf8_is_rejected(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([
            crate::command::ClientCommand::Send(
                crate::util::packet_to_vec(MPacket::Connack({
                    MConnack {
                        session_present: false,
                        connect_return_code: MConnectReturnCode::Accepted,
                    }
                }))
                .await?,
            ),
            crate::command::ClientCommand::Send(
                [
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
                ]
                .to_vec(),
            ),
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

    Ok(mk_report! {
        name: "Check if invalid UTF-8 is rejected",
        desc: "Invalid UTF-8 is not allowed per the MQTT spec. Any receiver should immediately close the connection upon receiving such a packet.",
        normative: "[MQTT-1.5.3-1, MQTT-1.5.3-2]",
        result,
        output
    })
}

async fn check_receiving_server_packet(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([
            crate::command::ClientCommand::Send(
                crate::util::packet_to_vec(MPacket::Connack({
                    MConnack {
                        session_present: false,
                        connect_return_code: MConnectReturnCode::Accepted,
                    }
                }))
                .await?,
            ),
            crate::command::ClientCommand::Send(
                crate::util::packet_to_vec(MPacket::Subscribe({
                    MSubscribe {
                        id: MPacketIdentifier(1),
                        subscriptions: MSubscriptionRequests {
                            count: 1,
                            data: b"a/b",
                        },
                    }
                }))
                .await?,
            ),
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

    Ok(mk_report! {
        name: "Check if invalid packets are rejected",
        desc: "Unexpected packets are a protocol error and the client MUST close the connection.",
        normative: "[MQTT-4.8.0-1]",
        result,
        output
    })
}

async fn check_invalid_first_packet_is_rejected(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::Send(
            crate::util::packet_to_vec(MPacket::Connect({
                MConnect {
                    protocol_name: MString { value: "foo" },
                    protocol_level: 0,
                    clean_session: true,
                    will: None,
                    username: None,
                    password: None,
                    keep_alive: 0,
                    client_id: MString { value: "client" },
                }
            }))
            .await?,
        )]);

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

    Ok(mk_report! {
        name: "Check if invalid first packet is rejected",
        desc: "The first packet from the server must be a ConnAck. Any other packet is invalid and the client should close the connection",
        normative: "[MQTT-3.2.0-1]",
        result,
        output
    })
}

async fn check_utf8_with_nullchar_is_rejected(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([
            crate::command::ClientCommand::Send(
                crate::util::packet_to_vec(MPacket::Connack({
                    MConnack {
                        session_present: false,
                        connect_return_code: MConnectReturnCode::Accepted,
                    }
                }))
                .await?,
            ),
            crate::command::ClientCommand::Send(vec![
                (MPacketKind::Publish {
                    dup: false,
                    qos: MQualityOfService::AtMostOnce,
                    retain: false,
                })
                .to_byte(),
                0b0000_0111, // Length
                // Now the variable header
                0b0000_0000,
                0b0000_0010,
                0x61,
                0x00,        // Zero byte
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
        name: String::from("Check if connection gets closed if UTF-8 string contains nullchar"),
        description: String::from(
            "The A UTF-8 encoded string MUST NOT include an encoding of the null character U+0000",
        ),
        normative_statement_number: String::from("[MQTT-1.5.3-2]"),
        result,
        output,
    })
}

async fn check_connack_flags_are_set_as_reserved(client_exe_path: &Path) -> miette::Result<Report> {
    let output = open_connection_with(client_exe_path)
        .await
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::Send(vec![
            0b0010_0000 | 0b0000_1000, // CONNACK + garbage
            0b0000_0010,               // Remaining length
            0b0000_0000,               // No session present
            0b0000_0000,               // Connection accepted
        ])]);

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
        name: String::from("Flag-Bit is set to 1 where it should be 0"),
        description: String::from(
            "CONNACK flag bits are marked as Reserved and must be set accordingly to spec",
        ),
        normative_statement_number: String::from("[MQTT-2.2.2-1]"),
        result,
        output,
    })
}
