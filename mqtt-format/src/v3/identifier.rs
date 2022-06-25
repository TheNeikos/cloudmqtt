use nom::{number::streaming::be_u16, Parser};

use super::MSResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MPacketIdentifier(pub u16);

pub fn mpacketidentifier(input: &[u8]) -> MSResult<'_, MPacketIdentifier> {
    be_u16.map(MPacketIdentifier).parse(input)
}
