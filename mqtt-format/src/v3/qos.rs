//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[cfg(feature = "std")]
use futures::{AsyncWrite, AsyncWriteExt};

#[cfg(feature = "std")]
use super::errors::MPacketWriteError;

use super::errors::MPacketHeaderError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MQualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

pub fn mquality_of_service(lower: u8) -> Result<MQualityOfService, MPacketHeaderError> {
    match lower {
        0b00 => Ok(MQualityOfService::AtMostOnce),
        0b01 => Ok(MQualityOfService::AtLeastOnce),
        0b10 => Ok(MQualityOfService::ExactlyOnce),
        inv_qos => Err(MPacketHeaderError::InvalidQualityOfService(inv_qos)),
    }
}

impl MQualityOfService {
    #[cfg(feature = "std")]
    pub async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer
            .write_all(match self {
                MQualityOfService::AtMostOnce => &[0x0],
                MQualityOfService::AtLeastOnce => &[0x1],
                MQualityOfService::ExactlyOnce => &[0x2],
            })
            .await?;
        Ok(())
    }
}
