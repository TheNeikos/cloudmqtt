//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v3::{packet::MConnect, strings::MString};

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct InvalidFirstPacketIsRejected;

#[async_trait::async_trait]
impl BehaviourTest for InvalidFirstPacketIsRejected {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    #[tracing::instrument(skip_all)]
    async fn execute(&self, mut input: Input, _output: Output) -> Result<(), miette::Error> {
        input
            .send_packet(MConnect {
                protocol_name: MString { value: "foo" },
                protocol_level: 0,
                clean_session: true,
                will: None,
                username: None,
                password: None,
                keep_alive: 0,
                client_id: MString { value: "client" },
            })
            .await?;
        Ok(())
    }

    fn report_name(&self) -> &str {
        "Check if invalid first packet is rejected"
    }

    fn report_desc(&self) -> &str {
        "The first packet from the server must be a ConnAck. Any other packet is invalid and the client should close the connection"
    }

    fn report_normative(&self) -> &str {
        "[MQTT-3.2.0-1]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
