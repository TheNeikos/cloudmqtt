//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ParserError;
use winnow::Bytes;

use super::write::WResult;
use super::write::WriteMqttPacket;
use super::MResult;

pub fn parse_bool(input: &mut &Bytes) -> MResult<bool> {
    winnow::binary::u8(input).and_then(|byte| match byte {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err({
            winnow::error::ErrMode::from_error_kind(input, winnow::error::ErrorKind::Verify)
        }),
    })
}

#[inline]
pub fn write_bool<W: WriteMqttPacket>(buffer: &mut W, b: bool) -> WResult<W> {
    buffer.write_byte(b as u8)
}
