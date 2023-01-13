//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::path::PathBuf;
use std::sync::Arc;

use futures::FutureExt;
use miette::IntoDiagnostic;

use mqtt_format::v3::packet::{MConnect, MPacket};

use mqtt_format::v3::strings::MString;

use crate::behaviour_test::BehaviourTest;
use crate::executable::ClientExecutable;
use crate::invariant::no_username_means_no_password::NoUsernameMeansNoPassword;
use crate::packet_invariant::PacketInvariant;
use crate::report::{Report, ReportResult};

pub async fn create_client_report(
    client_exe_path: PathBuf,
    parallelism: std::num::NonZeroUsize,
) -> miette::Result<Vec<Report>> {
    use futures::stream::StreamExt;

    let executable = ClientExecutable::new(client_exe_path);

    let reports = vec![
        check_first_packet_from_client_is_connect(&executable).boxed_local(),
        check_connect_packet_protocol_name(&executable).boxed_local(),
        check_connect_packet_reserved_flag_zero(&executable).boxed_local(),
        check_connect_flag_username_set_username_present(&executable).boxed_local(),
        check_connect_flag_password_set_password_present(&executable).boxed_local(),
        check_connect_flag_username_zero_means_password_zero(&executable).boxed_local(),
    ];

    let flows: Vec<Box<dyn BehaviourTest>> = vec![
        Box::new(crate::behaviour::WaitForConnect),
        Box::new(crate::behaviour::InvalidUtf8IsRejected),
        Box::new(crate::behaviour::ReceivingServerPacket),
        Box::new(crate::behaviour::InvalidFirstPacketIsRejected),
        Box::new(crate::behaviour::Utf8WithNullcharIsRejected),
        Box::new(crate::behaviour::ConnackFlagsAreSetAsReserved),
        Box::new(crate::behaviour::PublishQosZeroWithIdentFails),
        Box::new(crate::behaviour::PublishQos2IsAcked),
    ];

    let invariants: Vec<Arc<dyn PacketInvariant>> = vec![Arc::new(NoUsernameMeansNoPassword)];

    let mut collected_reports = Vec::with_capacity(flows.len());
    for flow in flows {
        let commands = flow.commands();

        let (client, input, mut output) = executable
            .call(&commands)
            .map(crate::command::Command::new)?
            .spawn()?;

        output.with_invariants(invariants.iter().cloned());

        let flow_result = flow.execute(input, output).await;
        let client_output = client.wait_with_output().await.into_diagnostic()?;

        collected_reports.push({
            Report {
                name: String::from(flow.report_name()),
                description: String::from(flow.report_desc()),
                normative_statement_number: String::from(flow.report_normative()),
                result: match flow_result {
                    Ok(_) => flow.translate_client_exit_code(client_output.status.success()),
                    Err(_e) => ReportResult::Failure,
                },
                output: Some(client_output.stdout),
            }
        })
    }

    Ok({
        futures::stream::iter(reports)
            .buffered(parallelism.get())
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .chain(collected_reports.into_iter())
            .collect()
    })
}

#[macro_export]
macro_rules! mk_report {
    (name: $name:literal,
     desc: $description:literal,
     normative: $normative:literal,
     $result:ident) => {
        Report {
            name: String::from($name),
            description: String::from($description),
            normative_statement_number: String::from($normative),
            $result,
            output: None,
        }
    };

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

#[macro_export]
macro_rules! wait_for_output {
    ($output:ident,
     timeout_ms: $timeout_ms:literal,
     out_success => $success:block,
     out_failure => $failure:block
    ) => {{
        #[allow(unused_imports)]
        use futures::Future;

        let (result, output) = match tokio::time::timeout(
            std::time::Duration::from_millis($timeout_ms),
            $output,
        )
        .await
        {
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

async fn check_first_packet_from_client_is_connect(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
                let packet =
                    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(bytes) {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                std::matches!(packet, MPacket::Connect { .. })
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
                let packet =
                    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(bytes) {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                match packet {
                    MPacket::Connect(connect) => connect.protocol_name == MString { value: "MQTT" },
                    _ => false,
                }
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
                bytes[0] == 0b0001_0000 // CONNECT packet with flags set to 0000
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
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
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
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
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
    let (client, _input, mut output) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
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
            }),
        )
        .await?;

    let output = client.wait_with_output();
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
