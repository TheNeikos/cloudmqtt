//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Various ways to parse MQTT integers
//!
//! All integers in MQTT are big-endian

use winnow::Bytes;
use winnow::Parser;
use winnow::combinator::trace;
use winnow::error::FromExternalError;
use winnow::token::take_while;

use super::MResult;
use super::write::WResult;
use super::write::WriteMqttPacket;

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

pub fn write_u16<W: WriteMqttPacket>(buffer: &mut W, u: u16) -> WResult<W> {
    buffer.write_u16(u)?;
    Ok(())
}

pub fn parse_u16_nonzero(input: &mut &Bytes) -> MResult<core::num::NonZeroU16> {
    let u: u16 = trace(
        "mqtt_u16",
        winnow::binary::u16(winnow::binary::Endianness::Big),
    )
    .parse_next(input)?;

    core::num::NonZeroU16::try_from(u)
        .map_err(|e| winnow::error::ErrMode::from_external_error(input, e))
}

#[inline]
pub fn write_u16_nonzero<W: WriteMqttPacket>(
    buffer: &mut W,
    u: core::num::NonZeroU16,
) -> WResult<W> {
    write_u16(buffer, u.get())
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

pub fn write_u32<W: WriteMqttPacket>(buffer: &mut W, u: u32) -> WResult<W> {
    buffer.write_u32(u)?;
    Ok(())
}

/// Parse a variable sized integer
///
/// Value range: `0..268_435_455`
/// The maximal value is smaller than a u32, so that type is used
///
#[doc = crate::v5::util::md_speclink!("_Toc3901011")]
pub fn parse_variable_u32<'a, I>(input: &mut I) -> MResult<u32>
where
    I: winnow::stream::StreamIsPartial + winnow::stream::Stream<Token = u8>,
    <I as winnow::stream::Stream>::Slice: IntoIterator<Item = &'a u8>,
{
    trace("mqtt_variable_u32", |input: &mut I| {
        let var_bytes = (
            take_while(0..=3, |b| b & 0b1000_0000 != 0)
                .context(winnow::error::StrContext::Label("front bytes")),
            winnow::binary::u8
                .verify(|b: &u8| b & 0b1000_0000 == 0)
                .context(winnow::error::StrContext::Label("last byte")),
        );
        let bytes = var_bytes.take().parse_next(input)?;

        let mut output: u32 = 0;

        for (exp, val) in bytes.into_iter().enumerate() {
            output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
        }

        Ok(output)
    })
    .parse_next(input)
}

pub const VARIABLE_INTEGER_MAX: u32 = 268_435_455;

#[inline]
pub const fn variable_u32_binary_size(u: u32) -> u32 {
    match u {
        0..=127 => 1,
        128..=16383 => 2,
        16384..=2_097_151 => 3,
        2_097_152..=268_435_455 => 4,
        _size => unreachable!(),
    }
}

pub fn write_variable_u32<W: WriteMqttPacket>(buffer: &mut W, u: u32) -> WResult<W> {
    match u {
        0..=127 => {
            buffer.write_byte(u as u8)?;
        }
        len @ 128..=16383 => {
            let first = (len % 128) | 0b1000_0000;
            let second = len / 128;
            buffer.write_byte(first as u8)?;
            buffer.write_byte(second as u8)?;
        }
        len @ 16384..=2_097_151 => {
            let first = (len % 128) | 0b1000_0000;
            let second = ((len / 128) % 128) | 0b1000_0000;
            let third = len / (128 * 128);

            buffer.write_byte(first as u8)?;
            buffer.write_byte(second as u8)?;
            buffer.write_byte(third as u8)?;
        }
        len @ 2_097_152..=268_435_455 => {
            let first = (len % 128) | 0b1000_0000;
            let second = ((len / 128) % 128) | 0b1000_0000;
            let third = ((len / (128 * 128)) % 128) | 0b1000_0000;
            let fourth = len / (128 * 128 * 128);

            buffer.write_byte(first as u8)?;
            buffer.write_byte(second as u8)?;
            buffer.write_byte(third as u8)?;
            buffer.write_byte(fourth as u8)?;
        }
        _size => {
            return Err(super::write::MqttWriteError::Invariant.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroUsize;

    use pretty_assertions::assert_eq;
    use winnow::Bytes;
    use winnow::Partial;
    use winnow::error::ErrMode;

    use crate::v5::integers::parse_u16;
    use crate::v5::integers::parse_u32;
    use crate::v5::integers::parse_variable_u32;
    use crate::v5::integers::write_variable_u32;
    use crate::v5::test::TestWriter;
    use crate::v5::write::WriteMqttPacket;

    #[test]
    fn check_incomplete_parsing() {
        let input = [0xFF];

        assert_eq!(
            parse_variable_u32(&mut Partial::new(Bytes::new(&input))).unwrap_err(),
            ErrMode::Incomplete(winnow::error::Needed::Size(NonZeroUsize::new(1).unwrap()))
        );

        let input = [0xFF, 0xFF, 0xFF];

        assert_eq!(
            parse_variable_u32(&mut Partial::new(Bytes::new(&input))).unwrap_err(),
            ErrMode::Incomplete(winnow::error::Needed::Size(NonZeroUsize::new(1).unwrap()))
        );

        let input = [0xFF, 0xFF, 0xFF, 0xFF];

        assert!(matches!(
            parse_variable_u32(&mut Partial::new(Bytes::new(&input))).unwrap_err(),
            ErrMode::Backtrack(_)
        ));
    }

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

    #[test]
    fn test_write_byte() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_byte(1).unwrap();

        assert_eq!(writer.buffer, &[1]);
    }

    #[test]
    fn test_write_two_bytes() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_u16(1).unwrap();

        assert_eq!(writer.buffer, &[0x00, 0x01]);
    }

    #[test]
    fn test_write_four_bytes() {
        let mut writer = TestWriter { buffer: Vec::new() };

        writer.write_u32(1).unwrap();

        assert_eq!(writer.buffer, &[0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn test_write_u32() {
        // step by some prime number
        for i in (0..268_435_455).step_by(271) {
            let mut writer = TestWriter { buffer: Vec::new() };

            writer.write_u32(i).unwrap();

            let out = parse_u32(&mut Bytes::new(&writer.buffer)).unwrap();
            assert_eq!(out, i);
        }
    }

    #[test]
    fn test_write_variable_u32() {
        // step by some prime number
        for i in (0..268_435_455).step_by(271) {
            let mut writer = TestWriter { buffer: Vec::new() };

            write_variable_u32(&mut writer, i).unwrap();

            let out = parse_variable_u32(&mut Bytes::new(&writer.buffer)).unwrap();
            assert_eq!(out, i);
        }
    }
}
