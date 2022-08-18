//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use bytes::{Bytes, BytesMut, BufMut};
use error::MqttError;
use mqtt_format::v3::{packet::MPacket, header::mfixedheader};
use tokio::io::AsyncReadExt;
use tracing::{debug, trace};

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

pub(crate) async fn read_one_packet<W: tokio::io::AsyncRead + Unpin>(
    mut reader: W,
) -> Result<MqttPacket, MqttError> {
    debug!("Reading a packet");

    let mut buffer = BytesMut::new();
    buffer.put_u16(reader.read_u16().await?);

    trace!(
        "Packet has reported size on first byte: 0b{size:08b} = {size}",
        size = buffer[1] & 0b0111_1111
    );
    if buffer[1] & 0b1000_0000 != 0 {
        trace!("Reading one more byte from size");
        buffer.put_u8(reader.read_u8().await?);
        if buffer[2] & 0b1000_0000 != 0 {
            trace!("Reading one more byte from size");
            buffer.put_u8(reader.read_u8().await?);
            if buffer[3] & 0b1000_0000 != 0 {
                trace!("Reading one more byte from size");
                buffer.put_u8(reader.read_u8().await?);
            }
        }
    }

    trace!("Parsing fixed header");

    let (_, header) = mfixedheader(&buffer).map_err(|_| MqttError::InvalidPacket)?;

    trace!("Reading remaining length: {}", header.remaining_length);

    buffer.reserve(header.remaining_length as usize);
    let mut buffer = buffer.limit(header.remaining_length as usize);

    reader.read_buf(&mut buffer).await?;

    let packet = crate::parse_packet(buffer.get_ref())?;

    trace!(?packet, "Received full packet");

    Ok(MqttPacket::new(buffer.into_inner().freeze()))
}
