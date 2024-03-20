use winnow::{Bytes, Parser};

use crate::v5::MResult;

pub struct MPingreq;

impl MPingreq {
    pub fn parse(input: &mut &Bytes) -> MResult<MPingreq> {
        winnow::combinator::eof.map(|_| MPingreq).parse_next(input)
    }
}
