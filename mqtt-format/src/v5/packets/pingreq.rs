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
