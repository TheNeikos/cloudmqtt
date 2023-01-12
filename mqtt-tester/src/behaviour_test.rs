//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use crate::{
    command::{Input, Output},
    executable::ClientExecutableCommand,
};

#[async_trait::async_trait]
pub trait BehaviourTest {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>>;

    async fn execute(&self, mut input: Input, mut output: Output) -> Result<(), miette::Error>;
}
