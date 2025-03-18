//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use crate::command::Input;
use crate::command::Output;
use crate::executable::ClientExecutableCommand;
use crate::report::ReportResult;

#[async_trait::async_trait]
pub trait BehaviourTest {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>>;

    async fn execute(
        &self,
        mut input: Input,
        mut output: Output,
    ) -> Result<ReportResult, miette::Error>;

    fn report_name(&self) -> &str;
    fn report_desc(&self) -> &str;
    fn report_normative(&self) -> &str;

    fn translate_client_exit_code(&self, success: bool) -> ReportResult {
        if success {
            ReportResult::Success
        } else {
            ReportResult::Failure
        }
    }
}
