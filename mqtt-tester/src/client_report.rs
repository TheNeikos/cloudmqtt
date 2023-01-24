//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::env::VarError;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use futures::FutureExt;

use miette::{Context, IntoDiagnostic};
use mqtt_format::v3::packet::{MConnect, MPacket};

use crate::behaviour_test::BehaviourTest;
use crate::executable::ClientExecutable;
use crate::invariant::connect_packet_protocol_name::ConnectPacketProtocolName;
use crate::invariant::no_username_means_no_password::NoUsernameMeansNoPassword;
use crate::packet_invariant::PacketInvariant;
use crate::report::{Report, ReportResult};

pub async fn create_client_report(
    client_exe_path: PathBuf,
    env: Vec<crate::Env>,
    parallelism: std::num::NonZeroUsize,
) -> miette::Result<Vec<Report>> {
    use futures::stream::StreamExt;

    let executable = ClientExecutable::new(client_exe_path, env);

    let reports = vec![
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
        Box::new(crate::behaviour::FirstPacketFromClientIsConnect),
    ];

    let invariants: Vec<Arc<dyn PacketInvariant>> = vec![
        Arc::new(NoUsernameMeansNoPassword),
        Arc::new(ConnectPacketProtocolName),
    ];

    futures::stream::iter(flows)
        .map(|flow| execute_flow(&executable, flow, &invariants).boxed_local())
        .chain(futures::stream::iter(reports))
        .buffered(parallelism.get())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
}

async fn execute_flow<'a>(
    executable: &'a ClientExecutable,
    flow: Box<dyn BehaviourTest>,
    invariants: &'a [Arc<dyn PacketInvariant>],
) -> miette::Result<Report> {
    tracing::debug!("Executing behaviour test: {:?}", flow.report_name());
    let commands = flow.commands();

    let (mut client, input, mut output, err_out) = executable
        .call(&commands)
        .map(crate::command::Command::new)
        .context("Creating client executable call")?
        .spawn()
        .context("Spawning client executable")?;

    tracing::debug!("Attaching invariants to flow output");
    output.with_invariants(invariants.iter().cloned());

    let duration = std::time::Duration::from_millis(100);
    let flow_fut = tokio::time::timeout(duration, flow.execute(input, output));
    let client_fut = tokio::time::timeout(duration, client.wait());

    let flow_fut = flow_fut.await;
    let flow_fut = match flow_fut {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(?e, flow = flow.report_name(), "Error: Flow future");
            return Ok(Report {
                name: String::from(flow.report_name()),
                description: String::from(flow.report_desc()),
                normative_statement_number: String::from(flow.report_normative()),
                result: ReportResult::Failure,
                output: None,
            });
        }
    };
    tracing::info!(flow = flow.report_name(), result = ?flow_fut, "Returned");

    let (stderr, client_fut) = tokio::join!(err_out.collect(std::time::Duration::from_millis(10)), client_fut);
    let stderr = stderr?;
    let client_fut = match client_fut {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(?e, flow = flow.report_name(), "Error: Client future");
            if let Some(procid) = client.id() {
                use nix::unistd::Pid;
                use nix::sys::signal::{self, Signal};

                tracing::debug!(?procid, "Sending SIGTERM to process");
                signal::kill(Pid::from_raw(procid.try_into().into_diagnostic()?), Signal::SIGTERM).into_diagnostic()?;
            }
            return Ok(Report {
                name: String::from(flow.report_name()),
                description: String::from(flow.report_desc()),
                normative_statement_number: String::from(flow.report_normative()),
                result: ReportResult::Failure,
                output: None,
            });
        }
    };

    let (result, output) = match (flow_fut, client_fut) {
        (Ok(flow_result), Ok(out)) => {
            tracing::debug!("Output ({}): {:?}", flow.report_name(), out);

            let res = flow.translate_client_exit_code(out.success());

            // If the client binary exited with success
            if res == ReportResult::Success {
                // We use the result of the flow itself, to return whether the test has succeeded
                (flow_result, Some(stderr))
            } else {
                // else, the binary exit result is sufficient (because it is either Inconclusive or
                // an error)
                (res, Some(stderr))
            }
        }
        (Err(e), _) => {
            tracing::error!("Error during behaviour testing: {:?}", e);
            (ReportResult::Failure, None)
        }
        (_, Err(e)) => {
            tracing::error!("Error during behaviour testing: {:?}", e);
            (ReportResult::Failure, None)
        }
    };

    Ok(Report {
        name: String::from(flow.report_name()),
        description: String::from(flow.report_desc()),
        normative_statement_number: String::from(flow.report_normative()),
        result,
        output,
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

#[tracing::instrument(skip_all)]
async fn check_connect_packet_reserved_flag_zero(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let (mut client, _input, mut output, err_out) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    let check_result = output
        .wait_and_check(
            &(|bytes: &[u8]| -> bool {
                bytes[0] == 0b0001_0000 // CONNECT packet with flags set to 0000
            }),
        )
        .await
        .context("Waiting for bytes to check")?;

    let stderr = err_out.collect(std::time::Duration::from_millis(10)).await?;
    let (result, output) =
        match tokio::time::timeout(std::time::Duration::from_millis(100), client.wait()).await {
            Ok(Ok(out)) => {
                let res = if out.success() {
                    check_result
                } else {
                    ReportResult::Inconclusive
                };

                (res, Some(stderr))
            }
            Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
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

#[tracing::instrument(skip_all)]
async fn check_connect_flag_username_set_username_present(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let (mut client, _input, mut output, err_out) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    let check_result = output
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
        .await
        .context("Waiting for bytes to check")?;

    let stderr = err_out.collect(std::time::Duration::from_millis(10)).await?;
    let (result, output) =
        match tokio::time::timeout(std::time::Duration::from_millis(100), client.wait()).await {
            Ok(Ok(out)) => {
                let res = if out.success() {
                    check_result
                } else {
                    ReportResult::Inconclusive
                };

                (res, Some(stderr))
            }
            Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
        };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for username is set, a username must be present",
        desc: "If the User Name Flag is set to 1, a user name MUST be present in the payload.",
        normative: "[MQTT-3.1.2-18, MQTT-3.1.2-19]",
        result,
        output
    })
}

#[tracing::instrument(skip_all)]
async fn check_connect_flag_password_set_password_present(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let (mut client, _input, mut output, err_out) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    let check_result = output
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
        .await
        .context("Waiting for bytes to check")?;

    let stderr = err_out.collect(std::time::Duration::from_millis(10)).await?;
    let (result, output) =
        match tokio::time::timeout(std::time::Duration::from_millis(100), client.wait()).await {
            Ok(Ok(out)) => {
                let res = if out.success() {
                    check_result
                } else {
                    ReportResult::Inconclusive
                };

                (res, Some(stderr))
            }
            Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
        };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for password is set, a password must be present",
        desc: "If the Password Flag is set to 1, a password MUST be present in the payload.",
        normative: "[MQTT-3.1.2-20, MQTT-3.1.2-21]",
        result,
        output
    })
}

#[tracing::instrument(skip_all)]
async fn check_connect_flag_username_zero_means_password_zero(
    executable: &ClientExecutable,
) -> miette::Result<Report> {
    let (mut client, _input, mut output, err_out) = executable
        .call(&[])
        .map(crate::command::Command::new)?
        .spawn()?;

    let check_result = output
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
        .await
        .context("Waiting for bytes to check")?;

    let stderr = err_out.collect(std::time::Duration::from_millis(10)).await?;
    let (result, output) =
        match tokio::time::timeout(std::time::Duration::from_millis(100), client.wait()).await {
            Ok(Ok(out)) => {
                let res = if out.success() {
                    check_result
                } else {
                    ReportResult::Inconclusive
                };

                (res, Some(stderr))
            }
            Ok(Err(_)) | Err(_) => (ReportResult::Failure, None),
        };

    Ok(mk_report! {
        name: "If the CONNECT packet flag for password is set, a password must be present",
        desc: "If the Password Flag is set to 1, a password MUST be present in the payload.",
        normative: "[MQTT-3.1.2-20, MQTT-3.1.2-21]",
        result,
        output
    })
}
