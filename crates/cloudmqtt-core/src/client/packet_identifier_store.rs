//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

pub trait PacketIdentifierStore {
    fn get_next_free(&mut self) -> Option<mqtt_format::v5::variable_header::PacketIdentifier>;
    fn release(&mut self, id: mqtt_format::v5::variable_header::PacketIdentifier);

    fn contains(&self, id: mqtt_format::v5::variable_header::PacketIdentifier) -> bool;
}

impl PacketIdentifierStore for usize {
    fn get_next_free(&mut self) -> Option<mqtt_format::v5::variable_header::PacketIdentifier> {
        for bit_index in 0..(usize::BITS as usize) {
            let mask = 0b1 << bit_index;
            if (*self & mask) == 0 {
                trace!(?bit_index, "Found a slot");
                *self |= mask;
                trace!("usize store is now {:0b}", *self);
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
        *self &= !mask;
    }

    fn contains(&self, id: mqtt_format::v5::variable_header::PacketIdentifier) -> bool {
        let id = id.0.get() as usize;
        if (id as u32 - 1) >= usize::BITS {
            return false;
        }

        let mask = 0b1 << (id - 1);
        trace!(bit_index = (id - 1), "Checking if contained");

        *self & mask != 0
    }
}
