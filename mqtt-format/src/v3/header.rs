use nom::{
    bits,
    bytes::streaming::{take, take_while_m_n},
    error::{Error, ErrorKind, FromExternalError},
    number::streaming::{be_u16, u8},
    sequence::tuple,
    IResult, Parser,
};

use super::strings::{mstring, MString};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MPacketHeader<'message> {
    kind: MPacketInfo<'message>,
    remaining_length: u32,
}

impl<'message> MPacketHeader<'message> {
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
pub enum MPacketHeaderError {
    #[error("An invalid Quality of Service (Qos) was supplied: {}", .0)]
    InvalidQualityOfService(u8),
    #[error("An invalid packet type was supplied: {}", .0)]
    InvalidPacketType(u8),
    #[error("The DUP flag was set in a publish message of Quality of Service (QoS) level 0.")]
    InvalidDupFlag,
    #[error("The packet length does not fit the remaining length")]
    InvalidPacketLength,
    #[error("The client sent an unsupported protocol name: {}", .0)]
    InvalidProtocolName(String),
    #[error("The client sent an unsupported protocol level: {}", .0)]
    InvalidProtocolLevel(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MPacketIdentifier(pub u16);

fn mpacketidentifier(input: &[u8]) -> IResult<&[u8], MPacketIdentifier> {
    be_u16.map(MPacketIdentifier).parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MQualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

fn mquality_of_service(lower: u8) -> Result<MQualityOfService, MPacketHeaderError> {
    match lower {
        0b00 => Ok(MQualityOfService::AtMostOnce),
        0b01 => Ok(MQualityOfService::AtLeastOnce),
        0b10 => Ok(MQualityOfService::ExactlyOnce),
        inv_qos => Err(MPacketHeaderError::InvalidQualityOfService(inv_qos)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MLastWill<'message> {
    topic: MString<'message>,
    payload: &'message [u8],
    qos: MQualityOfService,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MPacketInfo<'message> {
    Connect {
        protocol_name: MString<'message>,
        protocol_level: u8,
        clean_session: bool,
        will: Option<MLastWill<'message>>,
        username: Option<MString<'message>>,
        password: Option<&'message [u8]>,
        keep_alive: u16,
        client_id: MString<'message>,
    },
    Connack,
    Publish {
        dup: bool,
        qos: MQualityOfService,
        retain: bool,
        topic_name: MString<'message>,
        id: Option<MPacketIdentifier>,
        payload: &'message [u8],
    },
    Puback {
        id: MPacketIdentifier,
    },
    Pubrec {
        id: MPacketIdentifier,
    },
    Pubrel {
        id: MPacketIdentifier,
    },
    Pubcomp {
        id: MPacketIdentifier,
    },
    Subscribe {
        id: MPacketIdentifier,
    },
    Suback {
        id: MPacketIdentifier,
    },
    Unsubscribe {
        id: MPacketIdentifier,
    },
    Unsuback {
        id: MPacketIdentifier,
    },
    Pingreq,
    Pingresp,
    Disconnect,
}

fn mpayload(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, len) = be_u16(input)?;
    take(len)(input)
}

fn mpacketinfo(remaining_length: u32, input: &[u8]) -> IResult<&[u8], MPacketInfo> {
    let (input, (upper, lower)): (_, (u8, u8)) =
        bits::<_, _, Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::streaming::take(4usize),
            nom::bits::streaming::take(4usize),
        )))(input)?;

    let (input, info) = match (upper, lower) {
        (0b0001, 0b0000) => {
            let (input, protocol_name) = mstring(input)?;

            if &*protocol_name != "MQTT" {
                return Err(nom::Err::Error(Error::from_external_error(
                    input,
                    ErrorKind::MapRes,
                    MPacketHeaderError::InvalidProtocolName(protocol_name.to_string()),
                )));
            }

            let (input, protocol_level) = u8(input)?;

            if protocol_level != 4 {
                return Err(nom::Err::Error(Error::from_external_error(
                    input,
                    ErrorKind::MapRes,
                    MPacketHeaderError::InvalidProtocolLevel(protocol_level),
                )));
            }

            let (
                input,
                (user_name_flag, password_flag, will_retain, will_qos, will_flag, clean_session),
            ) = bits(tuple((
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(2usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
            )))(input)?;

            let (input, keep_alive) = be_u16(input)?;

            // Payload

            let (input, client_id) = mstring(input)?;

            let (input, will) = if will_flag == 1 {
                let (input, topic) = mstring(input)?;
                let (input, payload) = mpayload(input)?;

                (
                    input,
                    Some(MLastWill {
                        topic,
                        payload,
                        qos: mquality_of_service(will_qos).map_err(|e| {
                            nom::Err::Error(Error::from_external_error(input, ErrorKind::MapRes, e))
                        })?,
                    }),
                )
            } else {
                (input, None)
            };

            let (input, username) = if user_name_flag == 1 {
                mstring.map(Some).parse(input)?
            } else {
                (input, None)
            };

            let (input, password) = if password_flag == 1 {
                mpayload.map(Some).parse(input)?
            } else {
                (input, None)
            };

            (
                input,
                MPacketInfo::Connect {
                    protocol_name,
                    protocol_level,
                    clean_session: clean_session == 1,
                    will,
                    username,
                    password,
                    client_id,
                    keep_alive,
                },
            )
        }
        (0b0010, 0b0000) => (input, MPacketInfo::Connack),
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

            // Variable header

            let variable_header_start = input;

            let (input, topic_name) = mstring(input)?;

            let (input, id) = if qos != MQualityOfService::AtMostOnce {
                let (input, id) = mpacketidentifier(input)?;

                (input, Some(id))
            } else {
                (input, None)
            };

            if dup && qos == MQualityOfService::AtMostOnce {
                return Err(nom::Err::Error(Error::from_external_error(
                    input,
                    ErrorKind::MapRes,
                    MPacketHeaderError::InvalidDupFlag,
                )));
            }

            let variable_header_end = input;
            let variable_header_len = variable_header_start.len() - variable_header_end.len();

            // Payload

            let payload_length = match remaining_length.checked_sub(variable_header_len as u32) {
                Some(len) => len,
                None => {
                    return Err(nom::Err::Error(Error::from_external_error(
                        input,
                        ErrorKind::MapRes,
                        MPacketHeaderError::InvalidPacketLength,
                    )))
                }
            };
            let (input, payload) = take(payload_length)(input)?;

            (
                input,
                MPacketInfo::Publish {
                    qos,
                    dup,
                    retain,
                    id,
                    topic_name,
                    payload,
                },
            )
        }
        (0b0100, 0b0000) => (input, MPacketInfo::Puback),
        (0b0101, 0b0000) => (input, MPacketInfo::Pubrec),
        (0b0110, 0b0010) => (input, MPacketInfo::Pubrel),
        (0b1001, 0b0000) => (input, MPacketInfo::Pubcomp),
        (0b1000, 0b0000) => (input, MPacketInfo::Subscribe),
        (0b1001, 0b0010) => (input, MPacketInfo::Suback),
        (0b1010, 0b0000) => (input, MPacketInfo::Unsubscribe),
        (0b1011, 0b0010) => (input, MPacketInfo::Unsuback),
        (0b1100, 0b0000) => (input, MPacketInfo::Pingreq),
        (0b1101, 0b0000) => (input, MPacketInfo::Pingresp),
        (0b1110, 0b0000) => (input, MPacketInfo::Disconnect),
        (inv_type, _) => {
            return Err(nom::Err::Error(Error::from_external_error(
                input,
                ErrorKind::MapRes,
                MPacketHeaderError::InvalidPacketType(inv_type),
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

pub fn mfixedheader(input: &[u8]) -> IResult<&[u8], MPacketHeader> {
    let (input, kind) = mpacketinfo(input)?;
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
