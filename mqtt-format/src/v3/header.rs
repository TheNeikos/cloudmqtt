use nom::{
    bits,
    bytes::streaming::take_while_m_n,
    error::{Error, ErrorKind, FromExternalError},
    sequence::tuple,
    IResult, Parser,
};
use nom_supreme::ParserExt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MFixedHeader {
    kind: MPacketInfo,
    remaining_length: u32,
}

impl MFixedHeader {
    #[must_use]
    pub fn kind(&self) -> MPacketInfo {
        self.kind
    }

    #[must_use]
    pub fn remaining_length(&self) -> u32 {
        self.remaining_length
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MFixedHeaderError {
    #[error("An invalid Quality of Service (Qos) was supplied: {}", .0)]
    InvalidQualityOfService(u8),
    #[error("An invalid packet type was supplied: {}", .0)]
    InvalidPacketType(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MPacketInfo {
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
}

fn quality_of_service(lower: u8) -> Result<QualityOfService, MFixedHeaderError> {
    match lower {
        0b00 => Ok(QualityOfService::AtMostOnce),
        0b01 => Ok(QualityOfService::AtLeastOnce),
        0b10 => Ok(QualityOfService::ExactlyOnce),
        inv_qos => Err(MFixedHeaderError::InvalidQualityOfService(inv_qos)),
    }
}

fn mpacketinfo(input: &[u8]) -> IResult<&[u8], MPacketInfo> {
    let (input, (upper, lower)): (_, (u8, u8)) =
        bits::<_, _, Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::streaming::take(4usize),
            nom::bits::streaming::take(4usize),
        )))(input)?;

    let info = match (upper, lower) {
        (0b0000, 0b0000) => {
            return Err(nom::Err::Error(Error::from_external_error(
                input,
                ErrorKind::MapRes,
                MFixedHeaderError::InvalidPacketType(0b0000),
            )))
        }
        (0b0001, 0b0000) => MPacketInfo::Connect,
        (0b0010, 0b0000) => MPacketInfo::Connack,
        (0b0011, _) => MPacketInfo::Publish {
            dup: lower & 0b1000 == 1,
            qos: match quality_of_service(lower & 0b0110 >> 1) {
                Ok(qos) => qos,
                Err(e) => {
                    return Err(nom::Err::Error(Error::from_external_error(
                        input,
                        ErrorKind::MapRes,
                        e,
                    )))
                }
            },
            retain: lower & 0b0001 == 1,
        },
        (0b0100, 0b0000) => MPacketInfo::Puback,
        (0b0101, 0b0000) => MPacketInfo::Pubrec,
        (0b0110, 0b0010) => MPacketInfo::Pubrel,
        (0b1001, 0b0000) => MPacketInfo::Pubcomp,
        (0b1000, 0b0000) => MPacketInfo::Subscribe,
        (0b1001, 0b0010) => MPacketInfo::Suback,
        (0b1010, 0b0000) => MPacketInfo::Unsubscribe,
        (0b1011, 0b0010) => MPacketInfo::Unsuback,
        (0b1100, 0b0000) => MPacketInfo::Pingreq,
        (0b1101, 0b0000) => MPacketInfo::Pingresp,
        (0b1110, 0b0000) => MPacketInfo::Disconnect,
        (inv_type, _) => {
            return Err(nom::Err::Error(Error::from_external_error(
                input,
                ErrorKind::MapRes,
                MFixedHeaderError::InvalidPacketType(inv_type),
            )))
        }
    };

    Ok((input, info))
}

fn decode_variable_length(bytes: &[u8]) -> u32 {
    let mut output: u32 = 0;

    for (exp, val) in bytes.iter().enumerate() {
        output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
    }

    output
}

pub fn mfixedheader(input: &[u8]) -> IResult<&[u8], MFixedHeader> {
    let (input, kind) = mpacketinfo(input)?;
    let (input, remaining_length) = take_while_m_n(1, 4, |b| b & 0b1000_0000 != 0)
        .map(decode_variable_length)
        .parse(input)?;

    Ok((
        input,
        MFixedHeader {
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
