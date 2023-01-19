//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;
use mqtt_format::v3::{
    connect_return::MConnectReturnCode,
    identifier::MPacketIdentifier,
    packet::{MConnack, MPublish},
    qos::MQualityOfService,
    strings::MString,
};

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct PublishQos2IsAcked;

#[async_trait::async_trait]
impl BehaviourTest for PublishQos2IsAcked {
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
            .send_packet(MPublish {
                dup: false,
                qos: MQualityOfService::AtLeastOnce, // QoS 2
                retain: false,
                topic_name: MString { value: "a" },
                id: Some(MPacketIdentifier(1)),
                payload: &[0x00],
            })
            .await
            .context("Sending packet PUBLISH")?;

        Ok(ReportResult::Success)
    }

    fn report_name(&self) -> &str {
        "A PUBLISH packet is replied to with Puback with the same id"
    }

    fn report_desc(&self) -> &str {
        "A PUBACK, PUBREC or PUBREL Packet MUST contain the same Packet Identifier as the PUBLISH Packet that was originally sent."
    }

    fn report_normative(&self) -> &str {
        "[MQTT-2.3.1-6]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
