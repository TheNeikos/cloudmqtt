//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use super::write::MqttWriteError;
use super::write::WResult;
use super::write::WriteMqttPacket;

#[derive(Debug)]
pub struct TestWriter {
    pub buffer: Vec<u8>,
}

impl WriteMqttPacket for TestWriter {
    type Error = MqttWriteError;

    async fn write_byte(&mut self, u: u8) -> WResult<Self> {
        self.buffer.push(u);
        Ok(())
    }

    async fn write_slice(&mut self, u: &[u8]) -> WResult<Self> {
        self.buffer.extend(u);
        Ok(())
    }
}
