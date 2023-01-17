//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;
use mqtt_format::v3::{connect_return::MConnectReturnCode, packet::MConnack};

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct WaitForConnect;

#[async_trait::async_trait]
impl BehaviourTest for WaitForConnect {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    #[tracing::instrument(skip_all)]
    async fn execute(
        &self,
        mut input: Input,
        mut output: Output,
    ) -> Result<ReportResult, miette::Error> {
        let check_result = output
            .wait_and_check(
                &(|bytes: &[u8]| -> bool {
                    let connect_flags = if let Some(flags) = find_connect_flags(bytes) {
                        flags
                    } else {
                        return false;
                    };
                    tracing::trace!(?connect_flags, "Connect flags");

                    let username_flag_set = 0 != (connect_flags & 0b1000_0000); // Username flag
                    let password_flag_set = 0 != (connect_flags & 0b0100_0000); // Username flag
                    tracing::trace!(?username_flag_set, "username flag");
                    tracing::trace!(?password_flag_set, "password flag");

                    if username_flag_set {
                        !password_flag_set
                    } else {
                        true
                    }
                }),
            )
            .await
            .context("Waiting for bytes to check")?;

        tracing::debug!("Sending CONNACK");
        input
            .send_packet(MConnack {
                session_present: true,
                connect_return_code: MConnectReturnCode::Accepted,
            })
            .await
            .context("Sending packet: CONNACK")?;

        tracing::trace!(?check_result, "result of check");
        Ok(check_result)
    }

    fn report_name(&self) -> &str {
        "Wait for client to connect"
    }

    fn report_desc(&self) -> &str {
        "A client should send a CONNECT packet to connect to the server"
    }

    fn report_normative(&self) -> &str {
        "none"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Success
        } else {
            ReportResult::Failure
        }
    }
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
        tracing::trace!("Not a CONNECT packet");
        return None;
    }

    let str_len = getbyte!(3);
    tracing::trace!(?str_len, "Length of protocol name");

    let connect_flag_position = 4usize + (str_len as usize) + 2;
    tracing::trace!(?connect_flag_position, "Position of CONNECT flags");

    let flags = getbyte!(connect_flag_position);
    tracing::trace!(?flags, "CONNECT flags");

    Some(flags)
}
