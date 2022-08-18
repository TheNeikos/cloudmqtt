//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use bytes::Bytes;
use error::MqttError;
use mqtt_format::v3::packet::MPacket;

pub mod client;
pub mod error;
pub mod mqtt_stream;
pub mod packet_stream;

fn parse_packet(input: &[u8]) -> Result<MPacket<'_>, MqttError> {
    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(input) {
        Ok((_, packet)) => Ok(packet),

        Err(error) => {
            tracing::error!(?error, "Could not parse packet");
            Err(MqttError::InvalidPacket)
        }
    }
}

#[derive(Clone, Debug)]
pub struct MqttPacket {
    buffer: Bytes,
}

impl MqttPacket {
    pub(crate) fn new(buffer: Bytes) -> Self {
        Self { buffer }
    }

    pub fn get_packet(&self) -> Result<MPacket<'_>, MqttError> {
        parse_packet(&self.buffer)
    }
}
