//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Various ways to parse MQTT integers
//!
//! All integers in MQTT are big-endian

use winnow::combinator::trace;
use winnow::token::take_while;
use winnow::Bytes;
use winnow::Parser;

use super::write::WResult;
use super::write::WriteMqttPacket;
use super::MResult;

/// Parse a u16
///
/// MQTT expects their numbers in big-endian
#[doc = crate::v5::util::md_speclink!("_Toc3901008")]
pub fn parse_u16(input: &mut &Bytes) -> MResult<u16> {
    trace(
        "mqtt_u16",
        winnow::binary::u16(winnow::binary::Endianness::Big),
    )
    .parse_next(input)
}

pub async fn write_u16<W: WriteMqttPacket>(buffer: &mut W, u: u16) -> WResult<W> {
    buffer.write_u16(u.to_be()).await?;
    Ok(())
}

/// Parse a u32
///
/// MQTT expects their numbers in big-endian
#[doc = crate::v5::util::md_speclink!("_Toc3901009")]
pub fn parse_u32(input: &mut &Bytes) -> MResult<u32> {
    trace(
        "mqtt_u32",
        winnow::binary::u32(winnow::binary::Endianness::Big),
    )
    .parse_next(input)
}

pub async fn write_u32<W: WriteMqttPacket>(buffer: &mut W, u: u32) -> WResult<W> {
    buffer.write_u32(u.to_be()).await?;
    Ok(())
}

/// Parse a variable sized integer
///
/// Value range: `0..268_435_455`
/// The maximal value is smaller than a u32, so that type is used
///
#[doc = crate::v5::util::md_speclink!("_Toc3901011")]
pub fn parse_variable_u32(input: &mut &Bytes) -> MResult<u32> {
    trace("mqtt_variable_u32", |input: &mut &Bytes| {
        let var_bytes = (
            take_while(0..=3, |b| b & 0b1000_0000 != 0),
            winnow::binary::u8.verify(|b: &u8| b & 0b1000_0000 == 0),
        );
        let bytes: &[u8] = var_bytes.recognize().parse_next(input)?;

        let mut output: u32 = 0;

        for (exp, val) in bytes.iter().enumerate() {
            output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
        }

        Ok(output)
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::integers::parse_u16;
    use crate::v5::integers::parse_u32;
    use crate::v5::integers::parse_variable_u32;
    use crate::v5::test::TestWriter;
    use crate::v5::write::WriteMqttPacket;

    #[test]
    fn check_integer_parsing() {
        let input = 15u16.to_be_bytes();
        assert_eq!(parse_u16(&mut Bytes::new(&input)).unwrap(), 15);

        let input = 42u32.to_be_bytes();
        assert_eq!(parse_u32(&mut Bytes::new(&input)).unwrap(), 42);
    }

    #[test]
    fn check_variable_integers() {
        let input = [0x0];
        assert_eq!(parse_variable_u32(&mut Bytes::new(&input)).unwrap(), 0);

        let input = [0x7F];
        assert_eq!(parse_variable_u32(&mut Bytes::new(&input)).unwrap(), 127);

        let input = [0x80, 0x01];
        assert_eq!(parse_variable_u32(&mut Bytes::new(&input)).unwrap(), 128);

        let input = [0xFF, 0x7F];
        assert_eq!(parse_variable_u32(&mut Bytes::new(&input)).unwrap(), 16_383);

        let input = [0x80, 0x80, 0x01];
        assert_eq!(parse_variable_u32(&mut Bytes::new(&input)).unwrap(), 16_384);

        let input = [0xFF, 0xFF, 0x7F];
        assert_eq!(
            parse_variable_u32(&mut Bytes::new(&input)).unwrap(),
            2_097_151
        );

        let input = [0x80, 0x80, 0x80, 0x01];
        assert_eq!(
            parse_variable_u32(&mut Bytes::new(&input)).unwrap(),
            2_097_152
        );

        let input = [0xFF, 0xFF, 0xFF, 0x7F];
        assert_eq!(
            parse_variable_u32(&mut Bytes::new(&input)).unwrap(),
            268_435_455
        );

        let input = [0xFF, 0xFF, 0xFF, 0x8F];
        parse_variable_u32(&mut Bytes::new(&input)).unwrap_err();
    }

    #[tokio::test]
    async fn test_write_byte() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_byte(1).await.unwrap();

        assert_eq!(writer.buffer, &[1]);
    }

    #[tokio::test]
    async fn test_write_two_bytes() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_u16(1).await.unwrap();

        assert_eq!(writer.buffer, &[0x00, 0x01]);
    }

    #[tokio::test]
    async fn test_write_four_bytes() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_u32(1).await.unwrap();

        assert_eq!(writer.buffer, &[0x00, 0x00, 0x00, 0x01]);
    }
}
