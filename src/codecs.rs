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

use crate::packet::MqttPacket;
use crate::packet::MqttWriter;
use crate::packet::MqttWriterError;

#[derive(Debug, thiserror::Error)]
pub enum MqttPacketCodecError {
    #[error("A codec error")]
    Io(#[from] std::io::Error),

    #[error("An error occured while writing to a buffer")]
    Writer(#[from] MqttWriterError),

    #[error("A protocol error occurred")]
    Protocol,
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
            crate::packet::StableBytes(cart),
            |data| -> Result<_, MqttPacketCodecError> {
                FormatMqttPacket::parse_complete(data).map_err(|_| MqttPacketCodecError::Protocol)
            },
        )?;

        Ok(Some(MqttPacket { packet }))
    }
}

impl Encoder<MqttPacket> for MqttPacketCodec {
    type Error = MqttPacketCodecError;

    fn encode(
        &mut self,
        packet: MqttPacket,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.reserve(packet.get().binary_size() as usize);

        packet.get().write(&mut MqttWriter(dst))?;

        Ok(())
    }
}
