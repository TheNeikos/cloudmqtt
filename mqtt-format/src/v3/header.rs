use nom::{
    bits,
    bytes::complete::take_while_m_n,
    error::{Error, ErrorKind, FromExternalError},
    sequence::tuple,
    IResult, Parser,
};

use super::{
    errors::MPacketHeaderError,
    qos::{mquality_of_service, MQualityOfService},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MPacketHeader {
    kind: MPacketKind,
    remaining_length: u32,
}

impl MPacketHeader {
    #[must_use]
    pub fn kind(&self) -> MPacketKind {
        self.kind
    }

    #[must_use]
    pub fn remaining_length(&self) -> u32 {
        self.remaining_length
    }
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

    let (input, kind) = match (upper, lower) {
        (0b0001, 0b0000) => (input, MPacketKind::Connect),
        (0b0010, 0b0000) => (input, MPacketKind::Connack),
        (0b0011, lower) => {
            let dup = lower & 0b1000 == 1;
            let retain = lower & 0b0001 == 1;
            let qos = match mquality_of_service(lower & 0b0110 >> 1) {
                Ok(qos) => qos,
                Err(e) => {
                    return Err(nom::Err::Error(Error::from_external_error(
                        input,
                        ErrorKind::MapRes,
                        e,
                    )))
                }
            };
            (input, MPacketKind::Publish { qos, dup, retain })
        }
        (0b0100, 0b0000) => (input, MPacketKind::Puback),
        (0b0101, 0b0000) => (input, MPacketKind::Pubrec),
        (0b0110, 0b0010) => (input, MPacketKind::Pubrel),
        (0b1001, 0b0000) => (input, MPacketKind::Pubcomp),
        (0b1000, 0b0000) => (input, MPacketKind::Subscribe),
        (0b1001, 0b0010) => (input, MPacketKind::Suback),
        (0b1010, 0b0000) => (input, MPacketKind::Unsubscribe),
        (0b1011, 0b0010) => (input, MPacketKind::Unsuback),
        (0b1100, 0b0000) => (input, MPacketKind::Pingreq),
        (0b1101, 0b0000) => (input, MPacketKind::Pingresp),
        (0b1110, 0b0000) => (input, MPacketKind::Disconnect),
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

fn decode_variable_length(bytes: &[u8]) -> u32 {
    let mut output: u32 = 0;

    for (exp, val) in bytes.iter().enumerate() {
        output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
    }

    output
}

pub fn mfixedheader(input: &[u8]) -> IResult<&[u8], MPacketHeader> {
    let (input, kind) = mpacketkind(input)?;
    let (input, remaining_length) = take_while_m_n(1, 4, |b| b & 0b1000_0000 != 0)
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
    use super::decode_variable_length;

    #[test]
    fn check_variable_length_decoding() {
        let input = &[64];

        let output = decode_variable_length(input);
        assert_eq!(output, 64);

        let input = &[193, 2];

        let output = decode_variable_length(input);
        assert_eq!(output, 321);
    }
}
