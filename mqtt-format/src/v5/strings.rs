//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Various ways to parse MQTT Strings

use winnow::binary::length_take;
use winnow::error::ErrMode;
use winnow::error::FromExternalError;
use winnow::Bytes;
use winnow::Parser;

use super::integers::parse_u16;
use super::MResult;

/// Parse an UTF-8 String
///
/// MQTT expects that all Strings are UTF-8 encoded
///
#[doc = crate::v5::util::md_speclink!("_Toc3901010")]
pub fn parse_string<'i>(input: &mut &'i Bytes) -> MResult<&'i str> {
    winnow::combinator::trace("mqtt_string", |input: &mut &'i Bytes| {
        let maybe_str = length_take(parse_u16).parse_next(input)?;

        core::str::from_utf8(maybe_str)
            .map_err(|e| ErrMode::from_external_error(input, winnow::error::ErrorKind::Verify, e))
    })
    .parse_next(input)
}

/// Parse a pair of UTF-8 Strings
///
/// MQTT expects that all Strings are UTF-8 encoded
///
#[doc = crate::v5::util::md_speclink!("_Toc3901013")]
pub fn parse_string_pair<'i>(input: &mut &'i Bytes) -> MResult<(&'i str, &'i str)> {
    winnow::combinator::trace("mqtt_string_pair", |input: &mut &'i Bytes| {
        let first = parse_string(input)?;
        let second = parse_string(input)?;

        Ok((first, second))
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::strings::parse_string;

    #[test]
    fn check_simple_string() {
        let input = [0x0, 0x5, 0x41, 0xF0, 0xAA, 0x9B, 0x94];

        assert_eq!(parse_string(&mut Bytes::new(&input)).unwrap(), "A𪛔");
    }
}
