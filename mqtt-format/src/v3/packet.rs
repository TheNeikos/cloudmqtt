//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::pin::Pin;

use futures::AsyncWriteExt;
use nom::{
    bits, bytes::complete::take, error::FromExternalError, number::complete::be_u16,
    sequence::tuple, IResult, Parser,
};

use super::{
    connect_return::{mconnectreturn, MConnectReturnCode},
    errors::{MPacketHeaderError, MPacketWriteError},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl<'message> MPacket<'message> {
    pub async fn write_to<W: futures::AsyncWrite>(
        &self,
        mut writer: Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        macro_rules! write_remaining_length {
            ($writer:ident, $length:expr) => {
                match $length {
                    len @ 0..=127 => {
                        $writer.write_all(&[len as u8]).await?;
                    }
                    len @ 128..=16383 => {
                        let first = len % 128 | 0b1000_0000;
                        let second = len / 128;
                        $writer.write_all(&[first as u8, second as u8]).await?;
                    }
                    len @ 16384..=2_097_151 => {
                        let first = len % 128 | 0b1000_0000;
                        let second = (len / 128) % 128 | 0b1000_0000;
                        let third = len / (128 * 128);
                        $writer
                            .write_all(&[first as u8, second as u8, third as u8])
                            .await?;
                    }
                    len @ 2_097_152..=268_435_455 => {
                        let first = len % 128 | 0b1000_0000;
                        let second = (len / 128) % 128 | 0b1000_0000;
                        let third = (len / (128 * 128)) % 128 | 0b1000_0000;
                        let fourth = len / (128 * 128 * 128);
                        $writer
                            .write_all(&[first as u8, second as u8, third as u8, fourth as u8])
                            .await?;
                    }
                    size => {
                        return Err(MPacketWriteError::InvalidSize(size));
                    }
                }
            };
        }

        match self {
            MPacket::Connect {
                protocol_name,
                protocol_level,
                clean_session,
                will,
                username,
                password,
                keep_alive,
                client_id,
            } => {
                let packet_type = 0b0001_0000;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = 10
                    + MString::get_len(client_id)
                    + will.as_ref().map(MLastWill::get_len).unwrap_or_default()
                    + username.as_ref().map(MString::get_len).unwrap_or_default()
                    + password.as_ref().map(|p| 2 + p.len()).unwrap_or_default();

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable 1-6
                MString::write_to(protocol_name, &mut writer).await?;
                // Variable 7
                writer.write_all(&[*protocol_level]).await?;
                let connect_flags = bools_to_u8([
                    username.is_some(),
                    password.is_some(),
                    will.as_ref().map(|w| w.retain).unwrap_or_default(),
                    will.as_ref()
                        .map(|w| w.qos == MQualityOfService::ExactlyOnce)
                        .unwrap_or_default(),
                    will.as_ref()
                        .map(|w| w.qos != MQualityOfService::ExactlyOnce)
                        .unwrap_or_default(),
                    will.is_some(),
                    *clean_session,
                    false,
                ]);
                // Variable 8
                writer.write_all(&[connect_flags]).await?;
                // Variable 9-10
                writer.write_all(&keep_alive.to_be_bytes()).await?;

                // Payload Client
                MString::write_to(client_id, &mut writer).await?;

                // Payload Will
                if let Some(will) = will {
                    MString::write_to(&will.topic, &mut writer).await?;
                    writer
                        .write_all(&(will.payload.len() as u16).to_be_bytes())
                        .await?;
                    writer.write_all(will.payload).await?;
                }

                // Payload Username
                if let Some(username) = username {
                    MString::write_to(username, &mut writer).await?;
                }

                if let Some(password) = password {
                    writer
                        .write_all(&(password.len() as u16).to_be_bytes())
                        .await?;
                    writer.write_all(password).await?;
                }
            }
            MPacket::Connack {
                session_present,
                connect_return_code,
            } => {
                let packet_type = 0b0010_0000;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = 2;

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable 1-6
                writer
                    .write_all(&[*session_present as u8, *connect_return_code as u8])
                    .await?;
            }
            MPacket::Publish {
                dup,
                qos,
                retain,
                topic_name,
                id,
                payload,
            } => {
                let packet_type = 0b0011_0000;
                let dup_mask = if *dup { 0b0000_1000 } else { 0 };
                let qos_mask = qos.to_byte() << 1;
                let retain_mask = *retain as u8;

                // Header 1
                writer
                    .write_all(&[packet_type | dup_mask | qos_mask | retain_mask])
                    .await?;

                let remaining_length = MString::get_len(topic_name)
                    + id.as_ref().map(MPacketIdentifier::get_len).unwrap_or(0)
                    + payload.len();

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable Header
                MString::write_to(topic_name, &mut writer).await?;
                if let Some(id) = id {
                    MPacketIdentifier::write_to(id, &mut writer).await?;
                }
                writer.write_all(payload).await?;
            }
            MPacket::Puback { id } => {
                let packet_type = 0b0100_0000;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = 2;

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable 1-6
                id.write_to(&mut writer).await?;
            }
            MPacket::Pubrec { id } => {
                let packet_type = 0b0101_0000;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = 2;

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable 1-6
                id.write_to(&mut writer).await?;
            }
            MPacket::Pubrel { id: _ } => todo!(),
            MPacket::Pubcomp { id } => {
                let packet_type = 0b0111_0000;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = 2;

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable 1-6
                id.write_to(&mut writer).await?;
            }
            MPacket::Subscribe { id, subscriptions } => {
                let packet_type = 0b1000_0010;

                // Header 1
                writer.write_all(&[packet_type]).await?;

                let remaining_length = id.get_len() + subscriptions.get_len();

                // Header 2-5
                write_remaining_length!(writer, remaining_length);

                // Variable header

                id.write_to(&mut writer).await?;

                subscriptions.write_to(&mut writer).await?;
            }
            MPacket::Suback {
                id: _,
                subscription_acks: _,
            } => todo!(),
            MPacket::Unsubscribe {
                id: _,
                unsubscriptions: _,
            } => todo!(),
            MPacket::Unsuback { id: _ } => todo!(),
            MPacket::Pingreq => {
                let packet_type = 0b1100_0000;
                let variable_length = 0b0;

                // Header
                writer.write_all(&[packet_type, variable_length]).await?;
            }
            MPacket::Pingresp => todo!(),
            MPacket::Disconnect => todo!(),
        }

        Ok(())
    }
}

fn bools_to_u8(bools: [bool; 8]) -> u8 {
    (bools[0] as u8) << 7
        | (bools[1] as u8) << 6
        | (bools[2] as u8) << 5
        | (bools[3] as u8) << 4
        | (bools[4] as u8) << 3
        | (bools[5] as u8) << 2
        | (bools[6] as u8) << 1
        | (bools[7] as u8)
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

            if will_flag == 0 && will_qos != 0 {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InconsistentWillFlag,
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

    let data = nom::bytes::complete::take(header.remaining_length);

    let (input, packet) = data
        .and_then(|input| mpacketdata(header, input))
        .parse(input)?;

    Ok((input, packet))
}

#[cfg(test)]
mod tests {
    use crate::v3::{packet::MPacket, strings::MString, will::MLastWill};

    use super::mpacket;
    use std::pin::Pin;

    use pretty_assertions::assert_eq;

    #[test]
    fn check_complete_length() {
        let input = &[0b1110_0000, 0b0000_0000];

        let (rest, disc) = mpacket(input).unwrap();

        assert_eq!(rest, &[]);
        assert_eq!(disc, MPacket::Disconnect);
    }

    #[test]
    fn check_will_consistency() {
        let input = &[
            0b0001_0000,
            17,
            0x0,
            0x4, // String length
            b'M',
            b'Q',
            b'T',
            b'T',
            0x4,         // Level
            0b0000_1000, // Connect flags, with Will QoS = 1 and will flag = 0
            0x0,
            0x10, // Keel Alive in secs
            0x0,  // Client Identifier
            0x5,
            b'H',
            b'E',
            b'L',
            b'L',
            b'O',
        ];

        mpacket(input).unwrap_err();
    }

    #[tokio::test]
    async fn check_connect_roundtrip() {
        let input = &[
            0b0001_0000,
            37,
            0x0,
            0x4, // String length
            b'M',
            b'Q',
            b'T',
            b'T',
            0x4,         // Level
            0b1111_0110, // Connect flags
            0x0,
            0x10, // Keel Alive in secs
            0x0,  // Client Identifier
            0x5,
            b'H',
            b'E',
            b'L',
            b'L',
            b'O',
            0x0, // Will Topic
            0x5,
            b'W',
            b'O',
            b'R',
            b'L',
            b'D',
            0x0, // Will Payload
            0x1,
            0xFF,
            0x0,
            0x5, // Username
            b'A',
            b'D',
            b'M',
            b'I',
            b'N',
            0x0,
            0x1, // Password
            0xF0,
        ];

        let (_rest, conn) = mpacket(input).unwrap();

        assert_eq!(
            conn,
            MPacket::Connect {
                protocol_name: MString { value: "MQTT" },
                protocol_level: 4,
                clean_session: true,
                will: Some(MLastWill {
                    topic: MString { value: "WORLD" },
                    payload: &[0xFF],
                    qos: crate::v3::qos::MQualityOfService::ExactlyOnce,
                    retain: true
                }),
                username: Some(MString { value: "ADMIN" }),
                password: Some(&[0xF0]),
                keep_alive: 16,
                client_id: MString { value: "HELLO" }
            }
        );

        let mut buf = vec![];

        conn.write_to(Pin::new(&mut buf)).await.unwrap();

        assert_eq!(input, &buf[..]);
    }
}
