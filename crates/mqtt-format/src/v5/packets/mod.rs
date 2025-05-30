//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Handling of MQTT Control Packets

use winnow::Bytes;
use winnow::Parser;
use winnow::error::ContextError;
use winnow::error::ErrMode;

use self::auth::MAuth;
use self::connect::MConnect;
use self::disconnect::MDisconnect;
use self::pingreq::MPingreq;
use self::pingresp::MPingresp;
use self::puback::MPuback;
use self::pubcomp::MPubcomp;
use self::publish::MPublish;
use self::pubrec::MPubrec;
use self::pubrel::MPubrel;
use self::suback::MSuback;
use self::subscribe::MSubscribe;
use self::unsuback::MUnsuback;
use self::unsubscribe::MUnsubscribe;
use super::fixed_header::PacketType;
use super::write::WriteMqttPacket;
use crate::v5::MResult;
use crate::v5::fixed_header::MFixedHeader;
use crate::v5::packets::connack::MConnack;

pub mod auth;
pub mod connack;
pub mod connect;
pub mod disconnect;
pub mod pingreq;
pub mod pingresp;
pub mod puback;
pub mod pubcomp;
pub mod publish;
pub mod pubrec;
pub mod pubrel;
pub mod suback;
pub mod subscribe;
pub mod unsuback;
pub mod unsubscribe;

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Debug, Clone, PartialEq, derive_more::From, derive_more::TryInto)]
pub enum MqttPacket<'i> {
    Auth(MAuth<'i>),
    Connack(MConnack<'i>),
    Connect(MConnect<'i>),
    Disconnect(MDisconnect<'i>),
    Pingreq(MPingreq),
    Pingresp(MPingresp),
    Puback(MPuback<'i>),
    Pubcomp(MPubcomp<'i>),
    Publish(MPublish<'i>),
    Pubrec(MPubrec<'i>),
    Pubrel(MPubrel<'i>),
    Suback(MSuback<'i>),
    Subscribe(MSubscribe<'i>),
    Unsuback(MUnsuback<'i>),
    Unsubscribe(MUnsubscribe<'i>),
}

impl<'i> MqttPacket<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MqttPacket", |input: &mut &'i Bytes| {
            let fixed_header = MFixedHeader::parse(input)?;

            let parse_packet = |input: &mut &'i Bytes| match fixed_header.packet_type {
                PacketType::Connect => MConnect::parse(input).map(MqttPacket::from),
                PacketType::Connack => MConnack::parse(input).map(MqttPacket::from),
                PacketType::Publish { dup, qos, retain } => {
                    MPublish::parse(dup, qos, retain, input).map(MqttPacket::from)
                }
                PacketType::Puback => MPuback::parse(input).map(MqttPacket::from),
                PacketType::Pubrec => MPubrec::parse(input).map(MqttPacket::from),
                PacketType::Pubrel => MPubrel::parse(input).map(MqttPacket::from),
                PacketType::Pubcomp => MPubcomp::parse(input).map(MqttPacket::from),
                PacketType::Subscribe => MSubscribe::parse(input).map(MqttPacket::from),
                PacketType::Suback => MSuback::parse(input).map(MqttPacket::from),
                PacketType::Unsubscribe => MUnsubscribe::parse(input).map(MqttPacket::from),
                PacketType::Unsuback => MUnsuback::parse(input).map(MqttPacket::from),
                PacketType::Pingreq => MPingreq::parse(input).map(MqttPacket::from),
                PacketType::Pingresp => MPingresp::parse(input).map(MqttPacket::from),
                PacketType::Disconnect => MDisconnect::parse(input).map(MqttPacket::from),
                PacketType::Auth => MAuth::parse(input).map(MqttPacket::from),
            };

            let packet = winnow::binary::length_and_then(
                crate::v5::integers::parse_variable_u32,
                parse_packet,
            )
            .parse_next(input)?;

            Ok(packet)
        })
        .parse_next(input)
    }

    pub fn parse_complete(input: &'i [u8]) -> Result<Self, ErrMode<ContextError>> {
        Self::parse(&mut Bytes::new(input))
    }

    pub fn binary_size(&self) -> u32 {
        let header = MFixedHeader::binary_size();

        let packet_size = match self {
            MqttPacket::Auth(packet) => packet.binary_size(),
            MqttPacket::Connack(packet) => packet.binary_size(),
            MqttPacket::Connect(packet) => packet.binary_size(),
            MqttPacket::Disconnect(packet) => packet.binary_size(),
            MqttPacket::Pingreq(packet) => packet.binary_size(),
            MqttPacket::Pingresp(packet) => packet.binary_size(),
            MqttPacket::Puback(packet) => packet.binary_size(),
            MqttPacket::Pubcomp(packet) => packet.binary_size(),
            MqttPacket::Publish(packet) => packet.binary_size(),
            MqttPacket::Pubrec(packet) => packet.binary_size(),
            MqttPacket::Pubrel(packet) => packet.binary_size(),
            MqttPacket::Suback(packet) => packet.binary_size(),
            MqttPacket::Subscribe(packet) => packet.binary_size(),
            MqttPacket::Unsuback(packet) => packet.binary_size(),
            MqttPacket::Unsubscribe(packet) => packet.binary_size(),
        };

        header + crate::v5::integers::variable_u32_binary_size(packet_size) + packet_size
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> Result<(), W::Error> {
        match self {
            MqttPacket::Auth(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Auth,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Connack(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Connack,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Connect(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Connect,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Disconnect(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Disconnect,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Pingreq(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Pingreq,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Pingresp(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Pingresp,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Puback(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Puback,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Pubcomp(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Pubcomp,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Publish(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Publish {
                        dup: p.duplicate,
                        qos: p.quality_of_service,
                        retain: p.retain,
                    },
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Pubrec(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Pubrec,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Pubrel(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Pubrel,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Suback(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Suback,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Subscribe(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Subscribe,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Unsuback(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Unsuback,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
            MqttPacket::Unsubscribe(p) => {
                let fixed_header = MFixedHeader {
                    packet_type: PacketType::Unsubscribe,
                };
                fixed_header.write(buffer)?;
                crate::v5::integers::write_variable_u32(buffer, p.binary_size())?;
                p.write(buffer)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum MqttPacketKind {
    Auth,
    Connack,
    Connect,
    Disconnect,
    Pingreq,
    Pingresp,
    Puback,
    Pubcomp,
    Publish,
    Pubrec,
    Pubrel,
    Suback,
    Subscribe,
    Unsuback,
    Unsubscribe,
}

impl MqttPacket<'_> {
    pub fn get_kind(&self) -> MqttPacketKind {
        match self {
            MqttPacket::Auth(_) => MqttPacketKind::Auth,
            MqttPacket::Connack(_) => MqttPacketKind::Connack,
            MqttPacket::Connect(_) => MqttPacketKind::Connect,
            MqttPacket::Disconnect(_) => MqttPacketKind::Disconnect,
            MqttPacket::Pingreq(_) => MqttPacketKind::Pingreq,
            MqttPacket::Pingresp(_) => MqttPacketKind::Pingresp,
            MqttPacket::Puback(_) => MqttPacketKind::Puback,
            MqttPacket::Pubcomp(_) => MqttPacketKind::Pubcomp,
            MqttPacket::Publish(_) => MqttPacketKind::Publish,
            MqttPacket::Pubrec(_) => MqttPacketKind::Pubrec,
            MqttPacket::Pubrel(_) => MqttPacketKind::Pubrel,
            MqttPacket::Suback(_) => MqttPacketKind::Suback,
            MqttPacket::Subscribe(_) => MqttPacketKind::Subscribe,
            MqttPacket::Unsuback(_) => MqttPacketKind::Unsuback,
            MqttPacket::Unsubscribe(_) => MqttPacketKind::Unsubscribe,
        }
    }
}
