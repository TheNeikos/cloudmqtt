use winnow::error::{ErrMode, ParserError};
use winnow::Bytes;

use super::MResult;

#[derive(PartialEq)]
pub enum ProtocolLevel {
    V3,
    V5,
}

impl ProtocolLevel {
    pub fn parse<'i>(input: &mut &'i Bytes) -> MResult<Self> {
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
