//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;
use mqtt_format::v3::packet::MPacket;

use crate::behaviour_test::BehaviourTest;
use crate::command::Input;
use crate::command::Output;
use crate::executable::ClientExecutableCommand;
use crate::report::ReportResult;

pub struct FirstPacketFromClientIsConnect;

#[async_trait::async_trait]
impl BehaviourTest for FirstPacketFromClientIsConnect {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    #[tracing::instrument(skip_all)]
    async fn execute(
        &self,
        _input: Input,
        mut output: Output,
    ) -> Result<ReportResult, miette::Error> {
        let check_output = output
            .wait_and_check(
                &(|bytes: &[u8]| -> bool {
                    let packet = match nom::combinator::all_consuming(
                        mqtt_format::v3::packet::mpacket,
                    )(bytes)
                    {
                        Ok((_, packet)) => packet,
                        Err(_e) => return false,
                    };

                    std::matches!(packet, MPacket::Connect { .. })
                }),
            )
            .await
            .context("Waiting for bytes to check")?;

        Ok(check_output)
    }

    fn report_name(&self) -> &str {
        "First packet send by client must be CONNECT"
    }

    fn report_desc(&self) -> &str {
        "After a Network Connection is established by a Client to a Server, the first Packet sent from the Client to the Server MUST be a CONNECT Packet."
    }

    fn report_normative(&self) -> &str {
        "[MQTT-3.1.0-1]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
