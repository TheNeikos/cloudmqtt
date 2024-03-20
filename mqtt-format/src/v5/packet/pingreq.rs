use winnow::{Bytes, Parser};

use crate::v5::MResult;

pub struct MPingreq;

impl MPingreq {
    fn parse<'i>(input: &mut &'i Bytes) -> MResult<MPingreq> {
        winnow::combinator::eof.map(|_| MPingreq).parse_next(input)
    }
}
