//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Everything around parsing the fixed MQTT Header

use winnow::binary::bits::bits;
use winnow::error::ErrMode;
use winnow::error::FromExternalError;
use winnow::error::InputError;
use winnow::error::ParserError;
use winnow::Bytes;
use winnow::Parser;

use super::write::WResult;
use super::write::WriteMqttPacket;
use super::MResult;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PacketType {
    Connect,
    Connack,
    Publish {
        dup: bool,
        qos: crate::v5::qos::QualityOfService,
        retain: bool,
    },
    Puback,
    Pubrec,
    Pubrel,
    Pubcomp,
    Subscribe,
    Suback,
    Unsubscribe,
    Unsuback,
    Pingreq,
    Pingresp,
    Disconnect,
    Auth,
}

#[derive(Debug, PartialEq)]
pub struct MFixedHeader {
    pub packet_type: PacketType,
}

impl MFixedHeader {
    pub fn parse(input: &mut &Bytes) -> MResult<MFixedHeader> {
        let (packet_type, packet_flags): (u8, u8) = bits::<_, _, InputError<(_, usize)>, _, _>((
            winnow::binary::bits::take(4usize),
            winnow::binary::bits::take(4usize),
        ))
        .parse_next(input)
        .map_err(|_: ErrMode<InputError<_>>| {
            ErrMode::from_error_kind(input, winnow::error::ErrorKind::Slice)
        })?;

        let packet_type = match (packet_type, packet_flags) {
            (0, _) => {
                return Err(ErrMode::from_error_kind(
                    input,
                    winnow::error::ErrorKind::Verify,
                ))
            }
            (1, 0) => PacketType::Connect,
            (2, 0) => PacketType::Connack,
            (3, flags) => PacketType::Publish {
                dup: (0b1000 & flags) != 0,
                qos: crate::v5::qos::QualityOfService::try_from((flags & 0b0110) >> 1).map_err(
                    |e| ErrMode::from_external_error(input, winnow::error::ErrorKind::Verify, e),
                )?,
                retain: (0b0001 & flags) != 0,
            },
            (4, 0) => PacketType::Puback,
            (5, 0) => PacketType::Pubrec,
            (6, 0b0010) => PacketType::Pubrel,
            (7, 0) => PacketType::Pubcomp,
            (8, 0b0010) => PacketType::Subscribe,
            (9, 0) => PacketType::Suback,
            (10, 0b0010) => PacketType::Unsubscribe,
            (11, 0) => PacketType::Unsuback,
            (12, 0) => PacketType::Pingreq,
            (13, 0) => PacketType::Pingresp,
            (14, 0) => PacketType::Disconnect,
            (15, 0) => PacketType::Auth,
            _ => {
                return Err(ErrMode::from_error_kind(
                    input,
                    winnow::error::ErrorKind::Verify,
                ))
            }
        };

        Ok(MFixedHeader { packet_type })
    }

    pub fn binary_size() -> u32 {
        1
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        #[allow(clippy::identity_op)]
        let byte = match self.packet_type {
            PacketType::Connect => (1 << 4) | 0,
            PacketType::Connack => (2 << 4) | 0,
            PacketType::Publish { dup, qos, retain } => {
                let upper = 3 << 4;
                let lower = {
                    let dup = (dup as u8) << 3;
                    let qos = (qos as u8) << 1;
                    let retain = retain as u8;

                    dup | qos | retain
                };

                upper | lower
            }
            PacketType::Puback => (4 << 4) | 0,
            PacketType::Pubrec => (5 << 4) | 0,
            PacketType::Pubrel => (6 << 4) | 0b0010,
            PacketType::Pubcomp => (7 << 4) | 0,
            PacketType::Subscribe => (8 << 4) | 0b0010,
            PacketType::Suback => (9 << 4) | 0,
            PacketType::Unsubscribe => (10 << 4) | 0b0010,
            PacketType::Unsuback => (11 << 4) | 0,
            PacketType::Pingreq => (12 << 4) | 0,
            PacketType::Pingresp => (13 << 4) | 0,
            PacketType::Disconnect => (14 << 4) | 0,
            PacketType::Auth => (15 << 4) | 0,
        };

        buffer.write_byte(byte)
    }
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::fixed_header::MFixedHeader;

    #[test]
    fn check_fixed_header() {
        let input = &[0b0011_1010];

        assert_eq!(
            MFixedHeader::parse(&mut Bytes::new(&input)).unwrap(),
            MFixedHeader {
                packet_type: crate::v5::fixed_header::PacketType::Publish {
                    dup: true,
                    qos: crate::v5::qos::QualityOfService::AtLeastOnce,
                    retain: false
                },
            }
        )
    }
}
