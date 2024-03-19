use winnow::Bytes;

use crate::v5::MResult;

pub struct ApplicationMessagePayload<'i> {
    payload: &'i [u8]
}

impl<'i> ApplicationMessagePayload<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        crate::v5::bytes::parse_data(input).map(|payload| Self { payload })
    }
}


