//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;
use mqtt_format::v3::connect_return::MConnectReturnCode;
use mqtt_format::v3::header::MPacketKind;
use mqtt_format::v3::packet::MConnack;
use mqtt_format::v3::qos::MQualityOfService;

use crate::behaviour_test::BehaviourTest;
use crate::command::Input;
use crate::command::Output;
use crate::executable::ClientExecutableCommand;
use crate::report::ReportResult;

pub struct Utf8WithNullcharIsRejected;

#[async_trait::async_trait]
impl BehaviourTest for Utf8WithNullcharIsRejected {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    #[tracing::instrument(skip_all)]
    async fn execute(
        &self,
        mut input: Input,
        _output: Output,
    ) -> Result<ReportResult, miette::Error> {
        input
            .send_packet(MConnack {
                session_present: false,
                connect_return_code: MConnectReturnCode::Accepted,
            })
            .await
            .context("Sending packet CONNACK")?;

        input
            .send(&[
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
            ])
            .await
            .context("Sending broken packet PUBLISH")?;
        Ok(ReportResult::Success)
    }

    fn report_name(&self) -> &str {
        "Check if connection gets closed if UTF-8 string contains nullchar"
    }

    fn report_desc(&self) -> &str {
        "The A UTF-8 encoded string MUST NOT include an encoding of the null character U+0000"
    }

    fn report_normative(&self) -> &str {
        "[MQTT-1.5.3-2]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Success
        } else {
            ReportResult::Failure
        }
    }
}
