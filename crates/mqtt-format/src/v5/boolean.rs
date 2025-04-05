//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::error::ParserError;

use super::MResult;
use super::write::WriteMqttPacket;

pub fn parse_bool(input: &mut &Bytes) -> MResult<bool> {
    winnow::binary::u8(input).and_then(|byte| match byte {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(winnow::error::ErrMode::from_input(input)),
    })
}

#[inline]
pub fn write_bool<W: WriteMqttPacket>(buffer: &mut W, b: bool) -> Result<(), W::Error> {
    buffer.write_byte(b as u8)
}
