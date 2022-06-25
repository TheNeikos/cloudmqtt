use nom::{
    bits, bytes::streaming::take, error::FromExternalError, number::streaming::be_u16,
    sequence::tuple, IResult, Parser,
};

use super::{
    errors::MPacketHeaderError,
    identifier::{mpacketidentifier, MPacketIdentifier},
    qos::{mquality_of_service, MQualityOfService},
    strings::{mstring, MString},
    will::MLastWill,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MPacketData<'message> {
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

fn mpacketdata(remaining_length: u32, input: &[u8]) -> IResult<&[u8], MPacketData> {
    let (input, (upper, lower)): (_, (u8, u8)) =
        bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::streaming::take(4usize),
            nom::bits::streaming::take(4usize),
        )))(input)?;

    let (input, info) = match (upper, lower) {
        (0b0001, 0b0000) => {
            let (input, protocol_name) = mstring(input)?;

            if &*protocol_name != "MQTT" {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InvalidProtocolName(protocol_name.to_string()),
                )));
            }

            let (input, protocol_level) = nom::number::streaming::u8(input)?;

            if protocol_level != 4 {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
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
                            nom::Err::Error(nom::error::Error::from_external_error(
                                input,
                                nom::error::ErrorKind::MapRes,
                                e,
                            ))
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
                MPacketData::Connect {
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
        (0b0010, 0b0000) => (input, MPacketData::Connack),
        (0b0011, lower) => {
            let dup = lower & 0b1000 == 1;
            let retain = lower & 0b0001 == 1;
            let qos = match mquality_of_service(lower & 0b0110 >> 1) {
                Ok(qos) => qos,
                Err(e) => {
                    return Err(nom::Err::Error(nom::error::Error::from_external_error(
                        input,
                        nom::error::ErrorKind::MapRes,
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
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InvalidDupFlag,
                )));
            }

            let variable_header_end = input;
            let variable_header_len = variable_header_start.len() - variable_header_end.len();

            // Payload

            let payload_length = match remaining_length.checked_sub(variable_header_len as u32) {
                Some(len) => len,
                None => {
                    return Err(nom::Err::Error(nom::error::Error::from_external_error(
                        input,
                        nom::error::ErrorKind::MapRes,
                        MPacketHeaderError::InvalidPacketLength,
                    )))
                }
            };
            let (input, payload) = take(payload_length)(input)?;

            (
                input,
                MPacketData::Publish {
                    qos,
                    dup,
                    retain,
                    id,
                    topic_name,
                    payload,
                },
            )
        }
        (0b0100, 0b0000) => (input, MPacketData::Puback),
        (0b0101, 0b0000) => (input, MPacketData::Pubrec),
        (0b0110, 0b0010) => (input, MPacketData::Pubrel),
        (0b1001, 0b0000) => (input, MPacketData::Pubcomp),
        (0b1000, 0b0000) => (input, MPacketData::Subscribe),
        (0b1001, 0b0010) => (input, MPacketData::Suback),
        (0b1010, 0b0000) => (input, MPacketData::Unsubscribe),
        (0b1011, 0b0010) => (input, MPacketData::Unsuback),
        (0b1100, 0b0000) => (input, MPacketData::Pingreq),
        (0b1101, 0b0000) => (input, MPacketData::Pingresp),
        (0b1110, 0b0000) => (input, MPacketData::Disconnect),
        (inv_type, _) => {
            return Err(nom::Err::Error(nom::error::Error::from_external_error(
                input,
                nom::error::ErrorKind::MapRes,
                MPacketHeaderError::InvalidPacketType(inv_type),
            )))
        }
    };

    Ok((input, info))
}
