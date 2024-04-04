//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::num::NonZeroU16;

use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

use crate::codecs::MqttPacketCodec;
use crate::packet_identifier::PacketIdentifier;
use crate::string::MqttString;
use crate::transport::MqttConnection;

pub(super) struct ConnectState {
    pub(super) session_present: bool,
    pub(super) receive_maximum: Option<NonZeroU16>,
    pub(super) maximum_qos: Option<mqtt_format::v5::qos::MaximumQualityOfService>,
    pub(super) retain_available: Option<bool>,
    pub(super) topic_alias_maximum: Option<u16>,
    pub(super) maximum_packet_size: Option<u32>,
    pub(super) conn_write: FramedWrite<tokio::io::WriteHalf<MqttConnection>, MqttPacketCodec>,

    pub(super) conn_read_recv: futures::channel::oneshot::Receiver<
        FramedRead<tokio::io::ReadHalf<MqttConnection>, MqttPacketCodec>,
    >,

    pub(super) next_packet_identifier: std::num::NonZeroU16,
}

pub(super) struct SessionState {
    pub(super) client_identifier: MqttString,
    pub(super) outstanding_packets: OutstandingPackets,
}

pub(super) struct OutstandingPackets {
    pub(super) packet_ident_order: Vec<PacketIdentifier>,
    pub(super) outstanding_packets:
        std::collections::BTreeMap<PacketIdentifier, crate::packets::MqttPacket>,
}

impl OutstandingPackets {
    pub fn empty() -> Self {
        Self {
            packet_ident_order: Vec::new(),
            outstanding_packets: std::collections::BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, ident: PacketIdentifier, packet: crate::packets::MqttPacket) {
        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );

        self.packet_ident_order.push(ident);
        let removed = self.outstanding_packets.insert(ident, packet);

        debug_assert!(removed.is_none());
    }

    pub fn update_by_id(&mut self, ident: PacketIdentifier, packet: crate::packets::MqttPacket) {
        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );

        let removed = self.outstanding_packets.insert(ident, packet);

        debug_assert!(removed.is_some());
    }

    pub fn exists_outstanding_packet(&self, ident: PacketIdentifier) -> bool {
        self.outstanding_packets.contains_key(&ident)
    }

    pub fn iter_in_send_order(
        &self,
    ) -> impl Iterator<Item = (PacketIdentifier, &crate::packets::MqttPacket)> {
        self.packet_ident_order
            .iter()
            .flat_map(|id| self.outstanding_packets.get(id).map(|p| (*id, p)))
    }

    pub fn remove_by_id(&mut self, id: PacketIdentifier) {
        // Vec::retain() preserves order
        self.packet_ident_order.retain(|&elm| elm != id);
        self.outstanding_packets.remove(&id);

        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );
    }
}
