use futures::{AsyncWrite, AsyncWriteExt};
use nom::{number::complete::be_u16, Parser};

use super::{errors::MPacketWriteError, MSResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MPacketIdentifier(pub u16);

pub fn mpacketidentifier(input: &[u8]) -> MSResult<'_, MPacketIdentifier> {
    be_u16.map(MPacketIdentifier).parse(input)
}
impl MPacketIdentifier {
    pub(crate) async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer.write_all(&self.0.to_be_bytes()).await?;
        Ok(())
    }

    pub(crate) fn get_len(&self) -> usize {
        2
    }
}
