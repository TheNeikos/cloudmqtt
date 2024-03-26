//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Handling of MQTT Control Packets

use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::Bytes;
use winnow::Parser;

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
use super::write::WResult;
use super::write::WriteMqttPacket;
use crate::v5::fixed_header::MFixedHeader;
use crate::v5::packets::connack::MConnack;
use crate::v5::MResult;

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
#[derive(Clone, Debug)]
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

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
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

macro_rules! impl_try_from_packet {
    ($($kind:ty => $name:ident),*) => {
        $(
            impl<'i> From<$kind> for MqttPacket<'i> {
                fn from(from: $kind) -> Self {
                    MqttPacket::$name(from)
                }
            }
        )*
    };
}

impl_try_from_packet!(
    MAuth<'i> => Auth,
    MConnack<'i> => Connack,
    MConnect<'i> => Connect,
    MDisconnect<'i> => Disconnect,
    MPingreq => Pingreq,
    MPingresp => Pingresp,
    MPuback<'i> => Puback,
    MPubcomp<'i> => Pubcomp,
    MPublish<'i> => Publish,
    MPubrec<'i> => Pubrec,
    MPubrel<'i> => Pubrel,
    MSuback<'i> => Suback,
    MSubscribe<'i> => Subscribe,
    MUnsuback<'i> => Unsuback,
    MUnsubscribe<'i> => Unsubscribe
);
