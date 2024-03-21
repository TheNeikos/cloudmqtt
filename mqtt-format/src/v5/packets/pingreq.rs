//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901195")]
pub struct MPingreq;

impl MPingreq {
    pub fn parse(input: &mut &Bytes) -> MResult<MPingreq> {
        winnow::combinator::trace("MPingreq", winnow::combinator::eof.map(|_| Self))
            .parse_next(input)
    }

    pub async fn write<W: WriteMqttPacket>(&self, _buffer: &mut W) -> WResult<W> {
        Ok(())
    }
}
