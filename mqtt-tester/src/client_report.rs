//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::path::PathBuf;
use std::time::Duration;

use futures::FutureExt;
use mqtt_format::v3::connect_return::MConnectReturnCode;

use mqtt_format::v3::header::MPacketKind;
use mqtt_format::v3::identifier::MPacketIdentifier;
use mqtt_format::v3::packet::{MConnack, MConnect, MPacket, MPuback, MPublish, MSubscribe};

use mqtt_format::v3::qos::MQualityOfService;
use mqtt_format::v3::strings::MString;
use mqtt_format::v3::subscription_request::MSubscriptionRequests;

use crate::executable::ClientExecutable;
use crate::report::{Report, ReportResult};

pub async fn create_client_report(
    client_exe_path: PathBuf,
    parallelism: std::num::NonZeroUsize,
) -> miette::Result<Vec<Report>> {
    use futures::stream::StreamExt;

    let executable = ClientExecutable::new(client_exe_path);

    let reports = vec![
        check_invalid_utf8_is_rejected(&executable).boxed_local(),
        check_receiving_server_packet(&executable).boxed_local(),
        check_invalid_first_packet_is_rejected(&executable).boxed_local(),
        check_utf8_with_nullchar_is_rejected(&executable).boxed_local(),
        check_connack_flags_are_set_as_reserved(&executable).boxed_local(),
        check_publish_qos_zero_with_ident_fails(&executable).boxed_local(),
        check_publish_qos_2_is_acked(&executable).boxed_local(),
        check_first_packet_from_client_is_connect(&executable).boxed_local(),
        check_connect_packet_protocol_name(&executable).boxed_local(),
        check_connect_packet_reserved_flag_zero(&executable).boxed_local(),
        check_connect_flag_username_set_username_present(&executable).boxed_local(),
        check_connect_flag_password_set_password_present(&executable).boxed_local(),
        check_connect_flag_username_zero_means_password_zero(&executable).boxed_local(),
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

macro_rules! wait_for_output {
    ($output:ident,
     timeout_ms: $timeout_ms:literal,
     out_success => $success:block,
     out_failure => $failure:block
    ) => {{
        let (result, output) =
            match tokio::time::timeout(Duration::from_millis($timeout_ms), $output).await {
                Ok(Ok(out)) => (
                    if out.status.success() {
                        $success
                    } else {
                        $failure
                    },
                    Some(out.stderr),
                ),
                Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
            };

        (result, output)
    }};
}

async fn check_invalid_utf8_is_rejected(executable: &ClientExecutable) -> miette::Result<Report> {
    let output = executable
        .call([])
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

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Failure },
        out_failure => { ReportResult::Success }
    };

    Ok(mk_report! {
        name: "Check if invalid UTF-8 is rejected",
        desc: "Invalid UTF-8 is not allowed per the MQTT spec. Any receiver should immediately close the connection upon receiving such a packet.",
        normative: "[MQTT-1.5.3-1, MQTT-1.5.3-2]",
        result,
        output
    })
}

async fn check_receiving_server_packet(executable: &ClientExecutable) -> miette::Result<Report> {
    let output = executable
        .call([])
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

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Failure },
        out_failure => { ReportResult::Success }
    };

    Ok(mk_report! {
        name: "Check if invalid packets are rejected",
        desc: "Unexpected packets are a protocol error and the client MUST close the connection.",
        normative: "[MQTT-4.8.0-1]",
        result,
        output
    })
}

async fn check_invalid_first_packet_is_rejected(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
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

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Failure },
        out_failure => { ReportResult::Success }
    };

    Ok(mk_report! {
        name: "Check if invalid first packet is rejected",
        desc: "The first packet from the server must be a ConnAck. Any other packet is invalid and the client should close the connection",
        normative: "[MQTT-3.2.0-1]",
        result,
        output
    })
}

async fn check_utf8_with_nullchar_is_rejected(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
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

async fn check_connack_flags_are_set_as_reserved(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
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

async fn check_publish_qos_zero_with_ident_fails(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
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
                crate::util::packet_to_vec(MPacket::Publish({
                    MPublish {
                        dup: false,
                        qos: MQualityOfService::AtMostOnce, // QoS 0
                        retain: false,
                        topic_name: MString { value: "a" },
                        id: Some(MPacketIdentifier(1)),
                        payload: &[0x00],
                    }
                }))
                .await?,
            ),
        ]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Failure },
        out_failure => { ReportResult::Success }
    };

    Ok(mk_report! {
        name: "A PUBLISH packet with QoS zero must not contain a packet identifier",
        desc: "A PUBLISH Packet MUST NOT contain a Packet Identifier if its QoS value is set to 0.",
        normative: "[MQTT-2.3.1-5]",
        result,
        output
    })
}

async fn check_publish_qos_2_is_acked(executable: &ClientExecutable) -> miette::Result<Report> {
    let output = executable
        .call([])
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
                crate::util::packet_to_vec(MPacket::Publish({
                    MPublish {
                        dup: false,
                        qos: MQualityOfService::AtLeastOnce, // QoS 2
                        retain: false,
                        topic_name: MString { value: "a" },
                        id: Some(MPacketIdentifier(1)),
                        payload: &[0x00],
                    }
                }))
                .await?,
            ),
            crate::command::ClientCommand::WaitFor({
                crate::util::packet_to_vec(MPacket::Puback({
                    MPuback {
                        id: MPacketIdentifier(1),
                    }
                }))
                .await?
            }),
        ]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Failure }
    };

    Ok(mk_report! {
        name: "A PUBLISH packet is replied to with Puback with the same id",
        desc: "A PUBACK, PUBREC or PUBREL Packet MUST contain the same Packet Identifier as the PUBLISH Packet that was originally sent.",
        normative: "[MQTT-2.3.1-6]",
        result,
        output
    })
}

async fn check_first_packet_from_client_is_connect(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                let packet =
                    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(bytes) {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                std::matches!(packet, MPacket::Connect { .. })
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Failure }
    };

    Ok(mk_report! {
        name: "First packet received send by client must be CONNECT",
        desc: "After a Network Connection is established by a Client to a Server, the first Packet sent from the Client to the Server MUST be a CONNECT Packet.",
        normative: "[MQTT-3.1.0-1]",
        result,
        output
    })
}

async fn check_connect_packet_protocol_name(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                let packet =
                    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(bytes) {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                match packet {
                    MPacket::Connect(connect) => connect.protocol_name == MString { value: "MQTT" },
                    _ => false,
                }
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Inconclusive }
    };

    Ok(mk_report! {
        name: "Protocol name should be 'MQTT'",
        desc: "If the protocol name is incorrect the Server MAY disconnect the Client, or it MAY continue processing the CONNECT packet in accordance with some other specification. In the latter case, the Server MUST NOT continue to process the CONNECT packet in line with this specification",
        normative: "[MQTT-3.1.2-1]",
        result,
        output
    })
}

async fn check_connect_packet_reserved_flag_zero(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                bytes[0] == 0b0001_0000 // CONNECT packet with flags set to 0000
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Inconclusive }
    };

    Ok(mk_report! {
        name: "The CONNECT packet flags must be zero.",
        desc: "The Server MUST validate that the reserved flag in the CONNECT Control Packet is set to zero and disconnect the Client if it is not zero.",
        normative: "[MQTT-3.1.2-3]",
        result,
        output
    })
}

fn find_connect_flags(bytes: &[u8]) -> Option<u8> {
    macro_rules! getbyte {
        ($n:tt) => {
            if let Some(b) = bytes.get($n) {
                *b
            } else {
                return None;
            }
        };
    }

    if getbyte!(0) != 0b0001_0000 {
        return None;
    }

    let str_len = getbyte!(4);
    let connect_flag_position = 4usize + (str_len as usize) + 2;
    Some(getbyte!(connect_flag_position))
}

async fn check_connect_flag_username_set_username_present(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                let connect_flags = if let Some(flags) = find_connect_flags(bytes) {
                    flags
                } else {
                    return false;
                };

                if 0 != (connect_flags & 0b1000_0000) {
                    // username flag set
                    let packet = match nom::combinator::all_consuming(
                        mqtt_format::v3::packet::mpacket,
                    )(bytes)
                    {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                    match packet {
                        MPacket::Connect(MConnect { username, .. }) => username.is_some(),
                        _ => false,
                    }
                } else {
                    true
                }
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Inconclusive }
    };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for username is set, a username must be present",
        desc: "If the User Name Flag is set to 1, a user name MUST be present in the payload.",
        normative: "[MQTT-3.1.2-18, MQTT-3.1.2-19]",
        result,
        output
    })
}

async fn check_connect_flag_password_set_password_present(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                let connect_flags = if let Some(flags) = find_connect_flags(bytes) {
                    flags
                } else {
                    return false;
                };

                if 0 != (connect_flags & 0b0100_0000) {
                    // password flag set
                    let packet = match nom::combinator::all_consuming(
                        mqtt_format::v3::packet::mpacket,
                    )(bytes)
                    {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                    match packet {
                        MPacket::Connect(MConnect { password, .. }) => password.is_some(),
                        _ => false,
                    }
                } else {
                    true
                }
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Inconclusive }
    };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for password is set, a password must be present",
        desc: "If the Password Flag is set to 1, a password MUST be present in the payload.",
        normative: "[MQTT-3.1.2-20, MQTT-3.1.2-21]",
        result,
        output
    })
}

async fn check_connect_flag_username_zero_means_password_zero(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let output = executable
        .call([])
        .map(crate::command::Command::new)?
        .wait_for_write([crate::command::ClientCommand::WaitAndCheck(Box::new(
            |bytes: &[u8]| -> bool {
                let connect_flags = if let Some(flags) = find_connect_flags(bytes) {
                    flags
                } else {
                    return false;
                };

                let username_flag_set = 0 != (connect_flags & 0b1000_0000); // Username flag
                let password_flag_set = 0 != (connect_flags & 0b0100_0000); // Username flag

                if username_flag_set {
                    !password_flag_set
                } else {
                    true
                }
            },
        ))]);

    let (result, output) = wait_for_output! {
        output,
        timeout_ms: 100,
        out_success => { ReportResult::Success },
        out_failure => { ReportResult::Inconclusive }
    };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for password is set, a password must be present",
        desc: "If the Password Flag is set to 1, a password MUST be present in the payload.",
        normative: "[MQTT-3.1.2-20, MQTT-3.1.2-21]",
        result,
        output
    })
}
