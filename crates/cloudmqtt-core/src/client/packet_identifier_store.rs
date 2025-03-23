//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use crate::util::trace;

#[derive(PartialEq, Eq)]
pub enum PacketIdentifierUsage {
    Publish,
    NonPublish,
}

impl PacketIdentifierUsage {
    /// Returns `true` if the packet identifier usage is [`Publish`].
    ///
    /// [`Publish`]: PacketIdentifierUsage::Publish
    #[must_use]
    pub fn is_publish(&self) -> bool {
        matches!(self, Self::Publish)
    }
}

pub trait PacketIdentifierStore {
    fn get_next_free(
        &mut self,
        usage: PacketIdentifierUsage,
    ) -> Option<mqtt_format::v5::variable_header::PacketIdentifier>;
    fn release(&mut self, id: mqtt_format::v5::variable_header::PacketIdentifier);
    fn release_non_publish_slots(&mut self);

    fn contains(&self, id: mqtt_format::v5::variable_header::PacketIdentifier) -> bool;
}

#[derive(Debug, Default)]
pub struct UsizePacketIdentifierStore {
    slots: usize,
    is_publish: usize,
}

impl UsizePacketIdentifierStore {
    pub const fn new() -> Self {
        Self {
            slots: 0,
            is_publish: 0,
        }
    }
}

impl PacketIdentifierStore for UsizePacketIdentifierStore {
    fn get_next_free(
        &mut self,
        usage: PacketIdentifierUsage,
    ) -> Option<mqtt_format::v5::variable_header::PacketIdentifier> {
        for bit_index in 0..(usize::BITS as usize) {
            let mask = 0b1 << bit_index;
            if (self.slots & mask) == 0 {
                trace!(?bit_index, "Found a slot");
                self.slots |= mask;
                self.is_publish |= mask & (if usage.is_publish() { usize::MAX } else { 0 });
                trace!("usize store is now {:0b}", self.slots);
                return Some(mqtt_format::v5::variable_header::PacketIdentifier(
                    (bit_index as u16 + 1)
                        .try_into()
                        .expect("usize had more bits than fits into u16::MAX-1??"),
                ));
            }
        }
        None
    }

    fn release(&mut self, id: mqtt_format::v5::variable_header::PacketIdentifier) {
        let id = id.0.get() as usize;
        assert!((id as u32 - 1) < usize::BITS);

        let mask = 0b1 << (id - 1);
        trace!(bit_index = (id - 1), "Releasing index");
        self.slots &= !mask;
        self.is_publish &= !mask;
    }

    fn release_non_publish_slots(&mut self) {
        self.slots &= self.is_publish
    }

    fn contains(&self, id: mqtt_format::v5::variable_header::PacketIdentifier) -> bool {
        let id = id.0.get() as usize;
        if (id as u32 - 1) >= usize::BITS {
            return false;
        }

        let mask = 0b1 << (id - 1);
        trace!(bit_index = (id - 1), "Checking if contained");

        self.slots & mask != 0
    }
}

#[cfg(test)]
mod tests {
    use super::PacketIdentifierStore;
    use super::UsizePacketIdentifierStore;
    use crate::client::packet_identifier_store::PacketIdentifierUsage;

    #[test]
    fn check_slot_reuse_after_release() {
        let mut store = UsizePacketIdentifierStore::default();

        let first = store
            .get_next_free(PacketIdentifierUsage::NonPublish)
            .unwrap();
        let second = store.get_next_free(PacketIdentifierUsage::Publish).unwrap();
        assert_ne!(first, second);

        store.release_non_publish_slots();

        let third = store.get_next_free(PacketIdentifierUsage::Publish).unwrap();
        assert_eq!(first, third);

        store.release_non_publish_slots();

        let fourth = store
            .get_next_free(PacketIdentifierUsage::NonPublish)
            .unwrap();
        assert_eq!(fourth.0.get(), 3);
    }
}
