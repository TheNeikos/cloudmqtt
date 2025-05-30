//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Ways to parse MQTT byte data

use winnow::Bytes;
use winnow::Parser;
use winnow::binary::length_take;

use super::MResult;
use super::write::WriteMqttPacket;

pub fn parse_binary_data<'i>(input: &mut &'i Bytes) -> MResult<&'i [u8]> {
    winnow::combinator::trace("mqtt_binary_data", |input: &mut &'i Bytes| {
        length_take(super::integers::parse_u16).parse_next(input)
    })
    .parse_next(input)
}

pub fn write_binary_data<W: WriteMqttPacket>(buffer: &mut W, slice: &[u8]) -> Result<(), W::Error> {
    let slice_len = slice
        .len()
        .try_into()
        .map_err(|_| W::Error::from(super::write::MqttWriteError::Invariant))?;

    buffer.write_u16(slice_len)?;
    buffer.write_slice(slice)
}

#[inline]
pub fn binary_data_binary_size(data: &[u8]) -> u32 {
    (2 + data.len()) as u32
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::bytes::parse_binary_data;
    use crate::v5::bytes::write_binary_data;
    use crate::v5::test::TestWriter;

    #[test]
    fn check_binary_data() {
        let input = &[0x0, 0x2, 0x4, 0x2];

        assert_eq!(
            parse_binary_data(&mut Bytes::new(input)).unwrap(),
            &[0x4, 0x2]
        );
    }

    #[test]
    fn test_write_binary_data() {
        let mut writer = TestWriter { buffer: Vec::new() };
        let data = &[0xFF, 0xAB, 0x42, 0x13, 0x37, 0x69];

        write_binary_data(&mut writer, data).unwrap();
        let out = parse_binary_data(&mut Bytes::new(&writer.buffer)).unwrap();

        assert_eq!(out, data);
    }
}
