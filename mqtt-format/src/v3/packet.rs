use nom::{
    bits, bytes::streaming::take, error::FromExternalError, multi::many1,
    number::streaming::be_u16, sequence::tuple, IResult, Parser,
};

use super::{
    connect_return::{mconnectreturn, MConnectReturnCode},
    errors::MPacketHeaderError,
    header::{mfixedheader, MPacketHeader},
    identifier::{mpacketidentifier, MPacketIdentifier},
    qos::{mquality_of_service, MQualityOfService},
    strings::{mstring, MString},
    subscription_request::{msubscriptionrequest, msubscriptionrequests, MSubscriptionRequests},
    will::MLastWill,
    MSResult,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MPacket<'message> {
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
    Connack {
        session_present: bool,
        connect_return_code: MConnectReturnCode,
    },
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
        subscriptions: MSubscriptionRequests<'message>,
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

fn mpacketdata(fixed_header: MPacketHeader, input: &[u8]) -> IResult<&[u8], MPacket> {
    let (input, (upper, lower)): (_, (u8, u8)) =
        bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::streaming::take(4usize),
            nom::bits::streaming::take(4usize),
        )))(input)?;

    let (input, info) = match (upper, lower) {
        (1, 0b0000) => {
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
                (
                    user_name_flag,
                    password_flag,
                    will_retain,
                    will_qos,
                    will_flag,
                    clean_session,
                    reserved,
                ),
            ) = bits(tuple((
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(2usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
                nom::bits::streaming::take(1usize),
            )))(input)?;

            if reserved != 0 {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::ForbiddenReservedValue,
                )));
            }

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
                MPacket::Connect {
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
        (2, 0b0000) => {
            let (input, (reserved, session_present)) = bits(tuple((
                nom::bits::streaming::take(7usize),
                nom::bits::streaming::take(1usize),
            )))(input)?;

            if reserved != 0 {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::ForbiddenReservedValue,
                )));
            }

            let (input, connect_return_code) = mconnectreturn(input)?;

            (
                input,
                MPacket::Connack {
                    session_present: session_present == 1,
                    connect_return_code,
                },
            )
        }
        (3, lower) => {
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

            let payload_length = match fixed_header
                .remaining_length()
                .checked_sub(variable_header_len as u32)
            {
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
                MPacket::Publish {
                    qos,
                    dup,
                    retain,
                    id,
                    topic_name,
                    payload,
                },
            )
        }
        (4, 0b0000) => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Puback { id })
        }
        (5, 0b0000) => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubrec { id })
        }
        (6, 0b0010) => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubrel { id })
        }
        (7, 0b0000) => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubcomp { id })
        }
        (8, 0b0000) => {
            let (input, id) = mpacketidentifier(input)?;

            let (input, subscriptions) = msubscriptionrequests(input)?;

            (input, MPacket::Subscribe { id, subscriptions })
        }
        (9, 0b0010) => (input, MPacket::Suback),
        (10, 0b0000) => (input, MPacket::Unsubscribe),
        (11, 0b0010) => (input, MPacket::Unsuback),
        (12, 0b0000) => (input, MPacket::Pingreq),
        (13, 0b0000) => (input, MPacket::Pingresp),
        (14, 0b0000) => (input, MPacket::Disconnect),
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

fn mpacket(input: &[u8]) -> MSResult<'_, MPacket<'_>> {
    let (input, header) = mfixedheader(input)?;

    let (input, packet) = mpacketdata(header, input)?;

    Ok((input, packet))
}
