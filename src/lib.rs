//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::pin::Pin;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use futures::io::BufWriter;
use futures::AsyncWriteExt;
use mqtt_format::v3::errors::MPacketWriteError;
use mqtt_format::v3::{header::mfixedheader, packet::MPacket};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tracing::{debug, trace};
use yoke::Yoke;

pub mod client;
pub mod error;
pub mod mqtt_stream;
pub mod packet_stream;
pub mod server;

fn parse_packet(input: &[u8]) -> Result<MPacket<'_>, PacketIOError> {
    match nom::combinator::all_consuming(mqtt_format::v3::packet::mpacket)(input) {
        Ok((_, packet)) => Ok(packet),

        Err(error) => {
            tracing::error!(?error, "Could not parse packet");
            Err(PacketIOError::InvalidParsedPacket)
        }
    }
}

#[derive(Clone)]
pub struct MqttPacket {
    packet: Yoke<MPacket<'static>, Arc<Bytes>>,
}

impl std::fmt::Debug for MqttPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttPacket")
            .field("packet", self.packet.get())
            .finish()
    }
}

impl MqttPacket {
    pub fn get_packet(&self) -> &MPacket<'_> {
        self.packet.get()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PacketIOError {
    #[error("An error occured during an IO Operation")]
    Io(#[from] std::io::Error),
    #[error("An invalid packet was sent to the endpoint")]
    InvalidParsedPacket,
    #[error("A packet could not be written to its endpoint")]
    InvalidReceivedPacket(#[from] MPacketWriteError),
}

pub(crate) async fn read_one_packet<W: tokio::io::AsyncRead + Unpin>(
    mut reader: W,
) -> Result<MqttPacket, PacketIOError> {
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

    let (_, header) = mfixedheader(&buffer).map_err(|_| PacketIOError::InvalidParsedPacket)?;

    trace!("Reading remaining length: {}", header.remaining_length);

    buffer.reserve(header.remaining_length as usize);
    let mut buffer = buffer.limit(header.remaining_length as usize);

    reader.read_buf(&mut buffer).await?;

    let packet = MqttPacket {
        packet: Yoke::try_attach_to_cart(Arc::new(buffer.into_inner().freeze()), |data| {
            parse_packet(data)
        })?,
    };

    trace!(?packet, "Received full packet");

    Ok(packet)
}

pub(crate) async fn write_packet<W: AsyncWrite + std::marker::Unpin>(
    writer: &mut W,
    packet: MPacket<'_>,
) -> Result<(), PacketIOError> {
    let mut buf = BufWriter::new(writer.compat_write());
    trace!(?packet, "Sending packet");
    packet.write_to(Pin::new(&mut buf)).await?;
    buf.flush().await?;

    Ok(())
}
