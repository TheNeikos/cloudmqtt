use winnow::Bytes;

use crate::v5::MResult;

pub struct MPingresp;

impl MPingresp {
    pub fn parse(input: &mut &Bytes) -> MResult<Self> {
        winnow::combinator::eof(input).map(|_| Self)
    }
}
