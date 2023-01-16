//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use miette::Context;

use crate::{
    behaviour_test::BehaviourTest,
    command::{Input, Output},
    executable::ClientExecutableCommand,
    report::ReportResult,
};

pub struct ConnackFlagsAreSetAsReserved;

#[async_trait::async_trait]
impl BehaviourTest for ConnackFlagsAreSetAsReserved {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![]
    }

    #[tracing::instrument(skip_all)]
    async fn execute(&self, mut input: Input, _output: Output) -> Result<(), miette::Error> {
        input
            .send(&[
                0b0010_0000 | 0b0000_1000, // CONNACK + garbage
                0b0000_0010,               // Remaining length
                0b0000_0000,               // No session present
                0b0000_0000,               // Connection accepted
            ])
            .await
            .context("Sending bytes")?;
        Ok(())
    }

    fn report_name(&self) -> &str {
        "Flag-Bit is set to 1 where it should be 0"
    }

    fn report_desc(&self) -> &str {
        "CONNACK flag bits are marked as Reserved and must be set accordingly to spec"
    }

    fn report_normative(&self) -> &str {
        "[MQTT-2.2.2-1]"
    }

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Failure
        } else {
            ReportResult::Success
        }
    }
}
