use winnow::{
    binary::bits::bits,
    error::{ErrMode, FromExternalError, InputError, ParserError},
    Bytes, Parser,
};

use super::{integers::parse_variable_u32, MResult};

#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum QualityOfService {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

#[derive(Debug, PartialEq)]
pub enum PacketType {
    Connect,
    Connack,
    Publish {
        dup: bool,
        qos: QualityOfService,
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
    packet_type: PacketType,
    remaining_length: u32,
}

pub fn parse_fixed_header(input: &mut &Bytes) -> MResult<MFixedHeader> {
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
        (2, 0) => PacketType::Connect,
        (3, flags) => PacketType::Publish {
            dup: (0b1000 & flags) != 0,
            qos: QualityOfService::try_from((flags & 0b0110) >> 1).map_err(|e| {
                ErrMode::from_external_error(input, winnow::error::ErrorKind::Verify, e)
            })?,
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

    let remaining_length = parse_variable_u32(input)?;

    Ok(MFixedHeader {
        packet_type,
        remaining_length,
    })
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::fixed_header::{parse_fixed_header, MFixedHeader};

    #[test]
    fn check_fixed_header() {
        let input = &[0b0011_1010, 0xA];

        assert_eq!(
            parse_fixed_header(&mut Bytes::new(&input)).unwrap(),
            MFixedHeader {
                packet_type: crate::v5::fixed_header::PacketType::Publish {
                    dup: true,
                    qos: crate::v5::fixed_header::QualityOfService::AtLeastOnce,
                    retain: false
                },
                remaining_length: 10,
            }
        )
    }
}
