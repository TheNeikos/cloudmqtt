//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use futures::{AsyncWrite, AsyncWriteExt};
use nom::{number::complete::be_u16, Parser};

use super::{errors::MPacketWriteError, MSResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
