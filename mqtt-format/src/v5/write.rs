//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

pub type WResult<W> = core::result::Result<(), <W as WriteMqttPacket>::Error>;

#[derive(Debug)]
pub enum MqttWriteError {
    Invariant,
}

pub trait WriteMqttPacket: Send {
    type Error: From<MqttWriteError>;

    fn write_byte(&mut self, u: u8) -> impl core::future::Future<Output = WResult<Self>> + Send;
    fn write_slice(&mut self, u: &[u8])
        -> impl core::future::Future<Output = WResult<Self>> + Send;

    #[inline]
    fn write_u16(&mut self, u: u16) -> impl core::future::Future<Output = WResult<Self>> + Send {
        async move {
            self.write_byte((u >> 8) as u8).await?;
            self.write_byte(u as u8).await
        }
    }

    #[inline]
    fn write_u32(&mut self, u: u32) -> impl core::future::Future<Output = WResult<Self>> + Send {
        async move {
            self.write_byte((u >> 24) as u8).await?;
            self.write_byte((u >> 16) as u8).await?;
            self.write_byte((u >> 8) as u8).await?;
            self.write_byte(u as u8).await
        }
    }
}
