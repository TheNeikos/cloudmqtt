//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use mqtt_format::v5::packets::MqttPacket as FormatMqttPacket;
use tokio_util::bytes::BufMut;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use winnow::Partial;
use yoke::Yoke;

pub(crate) struct BytesMutWriter<'b>(pub(crate) &'b mut tokio_util::bytes::BytesMut);

#[derive(Debug, thiserror::Error)]
pub enum MqttWriterError {
    #[error("Could not write mqtt packet to buffer")]
    Write(mqtt_format::v5::write::MqttWriteError),
}

impl From<mqtt_format::v5::write::MqttWriteError> for MqttWriterError {
    fn from(v: mqtt_format::v5::write::MqttWriteError) -> Self {
        Self::Write(v)
    }
}

impl mqtt_format::v5::write::WriteMqttPacket for BytesMutWriter<'_> {
    type Error = MqttWriterError;

    fn write_byte(&mut self, u: u8) -> mqtt_format::v5::write::WResult<Self> {
        self.0.put_u8(u);
        Ok(())
    }

    fn write_slice(&mut self, u: &[u8]) -> mqtt_format::v5::write::WResult<Self> {
        self.0.extend_from_slice(u);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MqttPacket {
    packet: Yoke<FormatMqttPacket<'static>, Arc<[u8]>>,
}

impl MqttPacket {
    pub fn new(packet: FormatMqttPacket<'_>) -> MqttPacket {
        let mut buffer = tokio_util::bytes::BytesMut::with_capacity(packet.binary_size() as usize);

        packet.write(&mut BytesMutWriter(&mut buffer)).unwrap();

        let cart = Arc::from(buffer.to_vec());

        MqttPacket {
            packet: Yoke::attach_to_cart(cart, |data| {
                FormatMqttPacket::parse_complete(data).unwrap()
            }),
        }
    }

    pub fn get_packet(&self) -> &FormatMqttPacket<'_> {
        self.packet.get()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttPacketCodecError {
    #[error("A codec error")]
    Io(#[from] std::io::Error),

    #[error("An error occured while writing to a buffer")]
    Writer(#[from] MqttWriterError),

    #[error("A protocol error occurred")]
    Protocol,

    #[error("Could not parse during decoding due to: {:?}", .0)]
    Parsing(winnow::error::ErrMode<winnow::error::ContextError>),
}

pub(crate) struct MqttPacketCodec;

impl Decoder for MqttPacketCodec {
    type Item = MqttPacket;

    type Error = MqttPacketCodecError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // 1. Byte: FixedHeader
        // 2-5. Byte: Variable-Size

        tracing::trace!(len = src.len(), "Trying to decide packet");
        if src.len() < 2 {
            let additional = 2 - src.len();
            tracing::trace!(?additional, "Reserving more bytes");
            src.reserve(additional);
            return Ok(None);
        }

        let remaining_length =
            match mqtt_format::v5::integers::parse_variable_u32(&mut Partial::new(&src[1..])) {
                Ok(size) => size as usize,
                Err(winnow::error::ErrMode::Incomplete(winnow::error::Needed::Size(needed))) => {
                    tracing::trace!(additional = ?needed, "Reserving more bytes");
                    src.reserve(needed.into());
                    return Ok(None);
                }
                Err(winnow::error::ErrMode::Incomplete(winnow::error::Needed::Unknown)) => {
                    tracing::trace!(additional = 1, "Reserving more bytes");
                    src.reserve(1);
                    return Ok(None);
                }
                _ => {
                    tracing::trace!("Protocol error");
                    return Err(MqttPacketCodecError::Protocol);
                }
            };
        tracing::trace!(?remaining_length, "Found remaining packet length");

        let total_packet_length = 1
            + mqtt_format::v5::integers::variable_u32_binary_size(remaining_length as u32) as usize
            + remaining_length;
        tracing::trace!(?total_packet_length);

        if src.len() < total_packet_length {
            let additional = total_packet_length - src.len();
            tracing::trace!(additional, "Reserving more bytes");
            src.reserve(additional);
            return Ok(None);
        }

        let cart: Arc<[u8]> = Arc::from(src.split_to(total_packet_length).to_vec());

        let packet = Yoke::try_attach_to_cart(cart, |data| -> Result<_, MqttPacketCodecError> {
            FormatMqttPacket::parse_complete(data).map_err(MqttPacketCodecError::Parsing)
        })?;

        tracing::trace!(packet = ?packet.get(), "Finished decoding packet");
        Ok(Some(MqttPacket { packet }))
    }
}

impl Encoder<FormatMqttPacket<'_>> for MqttPacketCodec {
    type Error = MqttPacketCodecError;

    fn encode(
        &mut self,
        packet: FormatMqttPacket<'_>,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        tracing::trace!("Trying to encode packet");
        let size = packet.binary_size() as usize;
        dst.reserve(size);

        let pre_size = dst.len();
        packet.write(&mut BytesMutWriter(dst))?;
        let total_written = dst.len() - pre_size;

        debug_assert_eq!(
            total_written,
            size,
            "Expected written bytes and actual written bytes differ! This is a bug for the {:?} packet type.",
            packet.get_kind()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::SinkExt;
    use futures::StreamExt;
    use mqtt_format::v5::packets::MqttPacket as FormatMqttPacket;
    use mqtt_format::v5::packets::connect::MConnect;
    use mqtt_format::v5::packets::pingreq::MPingreq;
    use tokio_util::codec::Framed;

    use super::MqttPacketCodec;

    #[tokio::test]
    async fn simple_test_codec() {
        let (client, server) = tokio::io::duplex(100);
        let mut framed_client = Framed::new(client, MqttPacketCodec);
        let mut framed_server = Framed::new(server, MqttPacketCodec);

        let packet = FormatMqttPacket::Pingreq(MPingreq);

        let sent_packet = packet.clone();
        tokio::spawn(async move {
            framed_client.send(sent_packet).await.unwrap();
        });
        let recv_packet = framed_server.next().await.unwrap().unwrap();

        assert_eq!(packet, *recv_packet.get_packet());
    }

    #[tokio::test]
    async fn test_connect_codec() {
        let (client, server) = tokio::io::duplex(100);
        let mut framed_client = Framed::new(client, MqttPacketCodec);
        let mut framed_server = Framed::new(server, MqttPacketCodec);

        let packet = FormatMqttPacket::Connect(MConnect {
            client_identifier: "test",
            username: None,
            password: None,
            clean_start: false,
            will: None,
            properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
            keep_alive: 0,
        });

        let sent_packet = packet.clone();
        tokio::spawn(async move {
            framed_client.send(sent_packet.clone()).await.unwrap();
            framed_client.send(sent_packet).await.unwrap();
        });
        let recv_packet = framed_server.next().await.unwrap().unwrap();

        assert_eq!(packet, *recv_packet.get_packet());
    }
}
