//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::{Bytes, Parser};

use crate::v5::MResult;

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901195")]
pub struct MPingreq;

impl MPingreq {
    pub fn parse(input: &mut &Bytes) -> MResult<MPingreq> {
        winnow::combinator::eof.map(|_| MPingreq).parse_next(input)
    }
}
