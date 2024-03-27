//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v5::packets::MqttPacket as FormatMqttPacket;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use winnow::Partial;
use yoke::Yoke;

use crate::packets::MqttPacket;
use crate::packets::MqttWriterError;

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

        if src.len() < 2 {
            src.reserve(2 - src.len());
            return Ok(None);
        }

        let packet_size =
            match mqtt_format::v5::integers::parse_variable_u32(&mut Partial::new(&src[1..])) {
                Ok(size) => size,
                Err(winnow::error::ErrMode::Incomplete(winnow::error::Needed::Size(needed))) => {
                    src.reserve(needed.into());
                    return Ok(None);
                }
                _ => {
                    return Err(MqttPacketCodecError::Protocol);
                }
            };

        let remaining_length = packet_size as usize;

        let total_packet_length = 1
            + mqtt_format::v5::integers::variable_u32_binary_size(packet_size) as usize
            + remaining_length;

        if src.len() < total_packet_length {
            src.reserve(total_packet_length - src.len());
            return Ok(None);
        }

        let cart = src.split_to(total_packet_length).freeze();

        let packet = Yoke::try_attach_to_cart(
            crate::packets::StableBytes(cart),
            |data| -> Result<_, MqttPacketCodecError> {
                FormatMqttPacket::parse_complete(data).map_err(MqttPacketCodecError::Parsing)
            },
        )?;

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
        packet.write(&mut crate::packets::MqttWriter(dst))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::SinkExt;
    use futures::StreamExt;
    use mqtt_format::v5::packets::pingreq::MPingreq;
    use mqtt_format::v5::packets::MqttPacket as FormatMqttPacket;
    use tokio_util::codec::Framed;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    use super::MqttPacketCodec;
    use crate::transport::MqttConnection;

    #[tokio::test]
    async fn simple_test_codec() {
        let (client, server) = tokio::io::duplex(100);
        let mut framed_client =
            Framed::new(MqttConnection::Duplex(client.compat()), MqttPacketCodec);
        let mut framed_server =
            Framed::new(MqttConnection::Duplex(server.compat()), MqttPacketCodec);

        let packet = FormatMqttPacket::Pingreq(MPingreq);

        let sent_packet = packet.clone();
        tokio::spawn(async move {
            framed_client.send(sent_packet).await.unwrap();
        });
        let recv_packet = framed_server.next().await.unwrap().unwrap();

        assert_eq!(packet, *recv_packet.get());
    }
}
