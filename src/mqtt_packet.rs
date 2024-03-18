//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
use std::sync::Arc;

use bytes::Bytes;
use mqtt_format::v3::packet::MPacket;
use yoke::Yoke;
use yoke::Yokeable;

#[derive(Clone)]
pub struct MqttPacket {
    packet: Yoke<MPacket<'static>, Arc<Bytes>>,
}

impl std::fmt::Debug for MqttPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttPacket")
            .field("packet", self.packet.get())
            .finish()
    }
}

impl MqttPacket {
    pub fn get_packet(&self) -> &MPacket<'_> {
        self.packet.get()
    }

    pub(crate) fn new(packet: Yoke<MPacket<'static>, Arc<Bytes>>) -> MqttPacket {
        MqttPacket { packet }
    }
}

pub struct MqttSubPacket<P>
where
    P: for<'a> yoke::Yokeable<'a>,
    P: Clone,
{
    packet: Yoke<P, Arc<Bytes>>,
}

impl<P> Clone for MqttSubPacket<P>
where
    P: for<'a> yoke::Yokeable<'a>,
    P: Clone,
    for<'a> yoke::trait_hack::YokeTraitHack<<P as Yokeable<'a>>::Output>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            packet: self.packet.clone(),
        }
    }
}

impl<P> MqttSubPacket<P>
where
    P: for<'a> yoke::Yokeable<'a>,
    P: Clone,
{
    pub fn get_packet(&self) -> &<P as Yokeable<'_>>::Output {
        self.packet.get()
    }
}

pub trait IntoMqttSubPacket<Target>: Sized
where
    Target: for<'a> Yokeable<'a>,
    Target: Clone,
{
    fn into_mqtt_sub_packet(self) -> Option<MqttSubPacket<Target>>;
}

macro_rules! impl_sub_packet_conversion {
    ($name:ty) => {
        impl IntoMqttSubPacket<$name> for MqttPacket {
            fn into_mqtt_sub_packet(self) -> Option<MqttSubPacket<$name>> {
                let packet = self
                    .packet
                    .try_map_project(|packet, _| packet.try_into())
                    .ok()?;

                Some(MqttSubPacket { packet })
            }
        }
    };
}

impl_sub_packet_conversion!(mqtt_format::v3::packet::MConnect<'static>);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MConnack);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPublish<'static>);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPuback);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPubrec);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPubrel);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPubcomp);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MSubscribe<'static>);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MSuback<'static>);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MUnsuback);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPingreq);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MPingresp);
impl_sub_packet_conversion!(mqtt_format::v3::packet::MDisconnect);

impl<P> MqttSubPacket<P>
where
    P: for<'a> yoke::Yokeable<'a>,
    P: Clone,
{
    pub fn from_packet(packet: MqttPacket) -> Option<MqttSubPacket<P>>
    where
        MqttPacket: IntoMqttSubPacket<P>,
    {
        packet.into_mqtt_sub_packet()
    }
}
