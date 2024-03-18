//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::Bytes;

use super::MResult;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ProtocolLevel {
    V3,
    V5,
}

impl ProtocolLevel {
    pub fn parse(input: &mut &Bytes) -> MResult<Self> {
        match winnow::binary::u8(input)? {
            3 => Ok(Self::V3),
            5 => Ok(Self::V5),
            _ => Err(ErrMode::from_error_kind(
                input,
                winnow::error::ErrorKind::Verify,
            )),
        }
    }
}
