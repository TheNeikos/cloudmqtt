//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::ops::Deref;

use mqtt_format::v5::packets::MqttPacket as FormatMqttPacket;
use mqtt_format::v5::write::WriteMqttPacket;
use stable_deref_trait::StableDeref;
use tokio_util::bytes::BufMut;
use tokio_util::bytes::Bytes;
use tokio_util::bytes::BytesMut;
use yoke::CloneableCart;
use yoke::Yoke;

#[derive(Debug, Clone)]
pub(crate) struct StableBytes(pub(crate) Bytes);

impl Deref for StableBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

// SAFETY: StableBytes derefs to &[u8] through bytes, which is stable
unsafe impl StableDeref for StableBytes {}

// SAFETY: StableBytes clones the pointer to the inner slice, and as such is stable as well
unsafe impl CloneableCart for StableBytes {}

#[derive(Debug, Clone)]
pub struct MqttPacket {
    pub(crate) packet: Yoke<FormatMqttPacket<'static>, StableBytes>,
}

impl PartialEq for MqttPacket {
    fn eq(&self, other: &Self) -> bool {
        self.packet.get() == other.packet.get()
    }
}

impl MqttPacket {
    pub fn get(&self) -> &FormatMqttPacket<'_> {
        self.packet.get()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttWriterError {
    #[error("An error occured while writing an MqttPacket: {:?}", .0)]
    MqttWrite(mqtt_format::v5::write::MqttWriteError),
}

impl From<mqtt_format::v5::write::MqttWriteError> for MqttWriterError {
    fn from(value: mqtt_format::v5::write::MqttWriteError) -> Self {
        MqttWriterError::MqttWrite(value)
    }
}

pub struct MqttWriter<'a>(pub &'a mut BytesMut);

impl<'a> WriteMqttPacket for MqttWriter<'a> {
    type Error = MqttWriterError;

    fn write_byte(&mut self, u: u8) -> mqtt_format::v5::write::WResult<Self> {
        self.0.put_u8(u);

        Ok(())
    }

    fn write_slice(&mut self, u: &[u8]) -> mqtt_format::v5::write::WResult<Self> {
        self.0.put_slice(u);

        Ok(())
    }
}
