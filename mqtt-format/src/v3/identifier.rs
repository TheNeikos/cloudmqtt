//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[cfg(feature = "std")]
use futures::{AsyncWrite, AsyncWriteExt};
use nom::{number::complete::be_u16, Parser};

#[cfg(feature = "std")]
use super::errors::MPacketWriteError;
use super::MSResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MPacketIdentifier(pub u16);

pub fn mpacketidentifier(input: &[u8]) -> MSResult<'_, MPacketIdentifier> {
    be_u16.map(MPacketIdentifier).parse(input)
}

impl MPacketIdentifier {
    #[cfg(feature = "std")]
    pub(crate) async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer.write_all(&self.0.to_be_bytes()).await?;
        Ok(())
    }

    #[cfg(feature = "std")]
    pub(crate) fn get_len(&self) -> usize {
        2
    }
}
