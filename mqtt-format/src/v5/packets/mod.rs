//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

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

#[derive(Debug)]
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

        let packet =
            winnow::binary::length_and_then(crate::v5::integers::parse_variable_u32, parse_packet)
                .parse_next(input)?;

        Ok(packet)
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
