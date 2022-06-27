use nom::{
    bits,
    bytes::complete::take_while_m_n,
    error::{Error, ErrorKind, FromExternalError},
    sequence::tuple,
    IResult, Parser,
};
use nom_supreme::ParserExt;

use super::{
    errors::MPacketHeaderError,
    qos::{mquality_of_service, MQualityOfService},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MPacketHeader {
    pub kind: MPacketKind,
    pub remaining_length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MPacketKind {
    Connect,
    Connack,
    Publish {
        dup: bool,
        qos: MQualityOfService,
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
}

fn mpacketkind(input: &[u8]) -> IResult<&[u8], MPacketKind> {
    let (input, (upper, lower)): (_, (u8, u8)) =
        bits::<_, _, Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::complete::take(4usize),
            nom::bits::complete::take(4usize),
        )))(input)?;

    println!("Received packet kind: 0b{:b} 0b{:b}", upper, lower);

    let (input, kind) = match (upper, lower) {
        (1, 0b0000) => (input, MPacketKind::Connect),
        (2, 0b0000) => (input, MPacketKind::Connack),
        (3, lower) => {
            let dup = lower & 0b1000 != 0;
            let retain = lower & 0b0001 != 0;
            let qos = match mquality_of_service((lower & 0b0110) >> 1) {
                Ok(qos) => qos,
                Err(e) => {
                    println!("Got an invalid QOS!");
                    return Err(nom::Err::Error(Error::from_external_error(
                        input,
                        ErrorKind::MapRes,
                        e,
                    )));
                }
            };
            (input, MPacketKind::Publish { qos, dup, retain })
        }
        (4, 0b0000) => (input, MPacketKind::Puback),
        (5, 0b0000) => (input, MPacketKind::Pubrec),
        (6, 0b0010) => (input, MPacketKind::Pubrel),
        (7, 0b0000) => (input, MPacketKind::Pubcomp),
        (8, 0b0010) => (input, MPacketKind::Subscribe),
        (9, 0b0000) => (input, MPacketKind::Suback),
        (10, 0b0010) => (input, MPacketKind::Unsubscribe),
        (11, 0b0000) => (input, MPacketKind::Unsuback),
        (12, 0b0000) => (input, MPacketKind::Pingreq),
        (13, 0b0000) => (input, MPacketKind::Pingresp),
        (14, 0b0000) => (input, MPacketKind::Disconnect),
        (inv_type, _) => {
            return Err(nom::Err::Error(Error::from_external_error(
                input,
                ErrorKind::MapRes,
                MPacketHeaderError::InvalidPacketType(inv_type),
            )))
        }
    };

    Ok((input, kind))
}

pub fn decode_variable_length(bytes: &[u8]) -> u32 {
    let mut output: u32 = 0;

    for (exp, val) in bytes.iter().enumerate() {
        output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
    }

    output
}

pub fn mfixedheader(input: &[u8]) -> IResult<&[u8], MPacketHeader> {
    let (input, kind) = mpacketkind(input)?;
    println!("Packet kind: {:?}", kind);
    let (input, remaining_length) = nom::number::complete::u8
        .and(take_while_m_n(0, 3, |b| b & 0b1000_0000 != 0))
        .recognize()
        .map(decode_variable_length)
        .parse(input)?;

    Ok((
        input,
        MPacketHeader {
            kind,
            remaining_length,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::v3::{
        header::{MPacketHeader, MPacketKind},
        qos::MQualityOfService,
    };

    use super::{decode_variable_length, mfixedheader};

    #[test]
    fn check_variable_length_decoding() {
        let input = &[64];

        let output = decode_variable_length(input);
        assert_eq!(output, 64);

        let input = &[193, 2];

        let output = decode_variable_length(input);
        assert_eq!(output, 321);
    }

    #[test]
    fn check_header_publish_flags() {
        let input = &[0b0011_1101, 0];

        let (input, header) = mfixedheader(input).unwrap();

        assert_eq!(input, &[]);

        assert_eq!(
            header,
            MPacketHeader {
                remaining_length: 0,
                kind: MPacketKind::Publish {
                    dup: true,
                    qos: MQualityOfService::ExactlyOnce,
                    retain: true
                }
            }
        );
    }

    #[test]
    fn check_invalid_header_publish_flags() {
        let input = &[0b0011_1111, 0];

        mfixedheader(input).unwrap_err();
    }
}
