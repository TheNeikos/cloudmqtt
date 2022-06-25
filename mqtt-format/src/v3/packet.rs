use nom::{
    bits, bytes::complete::take, error::FromExternalError, number::complete::be_u16,
    sequence::tuple, IResult, Parser,
};

use super::{
    connect_return::{mconnectreturn, MConnectReturnCode},
    errors::MPacketHeaderError,
    header::{mfixedheader, MPacketHeader, MPacketKind},
    identifier::{mpacketidentifier, MPacketIdentifier},
    qos::{mquality_of_service, MQualityOfService},
    strings::{mstring, MString},
    subscription_acks::{msubscriptionacks, MSubscriptionAcks},
    subscription_request::{msubscriptionrequests, MSubscriptionRequests},
    unsubscription_request::{munsubscriptionrequests, MUnsubscriptionRequests},
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
        subscription_acks: MSubscriptionAcks<'message>,
    },
    Unsubscribe {
        id: MPacketIdentifier,
        unsubscriptions: MUnsubscriptionRequests<'message>,
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
    let (input, info) = match fixed_header.kind {
        MPacketKind::Connect => {
            let (input, protocol_name) = mstring(input)?;

            if &*protocol_name != "MQTT" {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InvalidProtocolName(protocol_name.to_string()),
                )));
            }

            let (input, protocol_level) = nom::number::complete::u8(input)?;

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
            ): (_, (u8, u8, u8, _, u8, u8, u8)) =
                bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(tuple((
                    nom::bits::complete::take(1usize),
                    nom::bits::complete::take(1usize),
                    nom::bits::complete::take(1usize),
                    nom::bits::complete::take(2usize),
                    nom::bits::complete::take(1usize),
                    nom::bits::complete::take(1usize),
                    nom::bits::complete::take(1usize),
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
                let retain = will_retain != 0;

                (
                    input,
                    Some(MLastWill {
                        topic,
                        payload,
                        retain,
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
        MPacketKind::Connack => {
            let (input, (reserved, session_present)): (_, (u8, u8)) =
                bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(tuple((
                    nom::bits::complete::take(7usize),
                    nom::bits::complete::take(1usize),
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
        MPacketKind::Publish { dup, qos, retain } => {
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
                .remaining_length
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
        MPacketKind::Puback => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Puback { id })
        }
        MPacketKind::Pubrec => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubrec { id })
        }
        MPacketKind::Pubrel => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubrel { id })
        }
        MPacketKind::Pubcomp => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Pubcomp { id })
        }
        MPacketKind::Subscribe => {
            let (input, id) = mpacketidentifier(input)?;

            let (input, subscriptions) = msubscriptionrequests(input)?;

            (input, MPacket::Subscribe { id, subscriptions })
        }
        MPacketKind::Suback => {
            let (input, id) = mpacketidentifier(input)?;

            let (input, subscription_acks) = msubscriptionacks(input)?;

            (
                input,
                MPacket::Suback {
                    id,
                    subscription_acks,
                },
            )
        }
        MPacketKind::Unsubscribe => {
            let (input, id) = mpacketidentifier(input)?;

            let (input, unsubscriptions) = munsubscriptionrequests(input)?;

            (
                input,
                MPacket::Unsubscribe {
                    id,
                    unsubscriptions,
                },
            )
        }
        MPacketKind::Unsuback => {
            let (input, id) = mpacketidentifier(input)?;

            (input, MPacket::Unsuback { id })
        }
        MPacketKind::Pingreq => (input, MPacket::Pingreq),
        MPacketKind::Pingresp => (input, MPacket::Pingresp),
        MPacketKind::Disconnect => (input, MPacket::Disconnect),
    };

    Ok((input, info))
}

pub fn mpacket(input: &[u8]) -> MSResult<'_, MPacket<'_>> {
    let (input, header) = mfixedheader(input)?;

    let data = nom::bytes::streaming::take(header.remaining_length);

    let (input, packet) = data
        .and_then(|input| mpacketdata(header, input))
        .parse(input)?;

    Ok((input, packet))
}

#[cfg(test)]
mod tests {
    use crate::v3::packet::MPacket;

    use super::mpacket;
    use std::num::NonZeroUsize;

    #[test]
    fn check_incomplete_length() {
        let input = &[0b1110_0000, 0b0000_0010];

        let res = mpacket(input).unwrap_err();

        assert_eq!(
            res,
            nom::Err::Incomplete(nom::Needed::Size(NonZeroUsize::new(2).unwrap())),
        );
    }

    #[test]
    fn check_complete_length() {
        let input = &[0b1110_0000, 0b0000_0000];

        let (rest, disc) = mpacket(input).unwrap();

        assert_eq!(rest, &[]);
        assert_eq!(disc, MPacket::Disconnect);
    }
}
