//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v3::{connect_return::MConnectReturnCode, packet::MConnack};

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct InvalidUtf8IsRejected;

#[async_trait::async_trait]
impl BehaviourTest for InvalidUtf8IsRejected {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    async fn execute(&self, mut input: Input, _output: Output) -> Result<(), miette::Error> {
        input
            .send_packet(MConnack {
                session_present: false,
                connect_return_code: MConnectReturnCode::Accepted,
            })
            .await?;

        input
            .send(&[
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
            ])
            .await?;
        Ok(())
    }

    fn report_name(&self) -> &str {
        "Check if invalid UTF-8 is rejected"
    }

    fn report_desc(&self) -> &str {
        "Invalid UTF-8 is not allowed per the MQTT spec. Any receiver should immediately close the connection upon receiving such a packet."
    }

    fn report_normative(&self) -> &str {
        "[MQTT-1.5.3-1, MQTT-1.5.3-2]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
