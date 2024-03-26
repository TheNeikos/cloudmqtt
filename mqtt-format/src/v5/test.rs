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

    fn write_byte(&mut self, u: u8) -> WResult<Self> {
        self.buffer.push(u);
        Ok(())
    }

    fn write_slice(&mut self, u: &[u8]) -> WResult<Self> {
        self.buffer.extend(u);
        Ok(())
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }
}

macro_rules! make_roundtrip_test {
    ($name:ident $def:tt) => {
        let mut writer = $crate::v5::test::TestWriter { buffer: Vec::new() };
        let instance = $name $def;
        instance.write(&mut writer).unwrap();
        let output = $name::parse(&mut winnow::Bytes::new(&writer.buffer)).unwrap();
        assert_eq!(instance, output);
    }
}
pub(crate) use make_roundtrip_test;
