use winnow::Bytes;

use crate::v5::MResult;

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901200")]
pub struct MPingresp;

impl MPingresp {
    pub fn parse(input: &mut &Bytes) -> MResult<Self> {
        winnow::combinator::eof(input).map(|_| Self)
    }
}
