//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::MResult;
use crate::v5::write::WriteMqttPacket;

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901195")]
pub struct MPingreq;

impl MPingreq {
    pub fn parse(input: &mut &Bytes) -> MResult<MPingreq> {
        winnow::combinator::trace("MPingreq", winnow::combinator::eof.map(|_| Self))
            .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        0
    }

    pub fn write<W: WriteMqttPacket>(&self, _buffer: &mut W) -> Result<(), W::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::pingreq::MPingreq;

    #[test]
    fn test_roundtrip_pingreq() {
        let mut writer = crate::v5::test::TestWriter { buffer: Vec::new() };
        let instance = MPingreq;
        instance.write(&mut writer).unwrap();
        let output = MPingreq::parse(&mut winnow::Bytes::new(&writer.buffer)).unwrap();
        assert_eq!(instance, output);
    }
}
