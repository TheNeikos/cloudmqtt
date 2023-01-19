//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;
use mqtt_format::v3::{
    connect_return::MConnectReturnCode,
    identifier::MPacketIdentifier,
    packet::{MConnack, MSubscribe},
    subscription_request::MSubscriptionRequests,
};

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct ReceivingServerPacket;

#[async_trait::async_trait]
impl BehaviourTest for ReceivingServerPacket {
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
            .send_packet(MSubscribe {
                id: MPacketIdentifier(1),
                subscriptions: MSubscriptionRequests {
                    count: 1,
                    data: b"a/b",
                },
            })
            .await
            .context("Sending packet SUBSCRIBE")?;
        Ok(ReportResult::Success)
    }

    fn report_name(&self) -> &str {
        "Check if invalid packets are rejected"
    }

    fn report_desc(&self) -> &str {
        "Unexpected packets are a protocol error and the client MUST close the connection."
    }

    fn report_normative(&self) -> &str {
        "[MQTT-4.8.0-1]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
