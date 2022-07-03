use std::sync::{atomic::AtomicU16, Arc};

use bytes::Bytes;
use dashmap::DashMap;
use mqtt_format::v3::packet::MPacket;

use crate::{error::MqttError, parse_packet};

pub struct PacketStorage {
    last_id: AtomicU16,
    messages: Arc<DashMap<u16, MqttPacket>>,
}

impl PacketStorage {
    pub fn new() -> Self {
        PacketStorage {
            last_id: AtomicU16::new(0),
            messages: Arc::new(DashMap::new()),
        }
    }

    pub fn push_to_storage(&self, bytes: MqttPacket) -> u16 {
        let id: u16 = self.get_next_free_id();

        self.messages
            .insert(id, bytes)
            .expect("The slot should not have been occupied");

        id
    }

    fn get_next_free_id(&self) -> u16 {
        for _ in 0..=u16::MAX {
            // Fetch add wraps around on overflow
            let id = self
                .last_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if self.messages.contains_key(&id) {
                continue;
            }

            return id;
        }

        panic!("Could not find a free packet id")
    }

    pub fn get_from_storage(&self, id: u16) -> Option<MqttPacket> {
        self.messages.get(&id).map(|e| e.clone())
    }
}

#[derive(Clone, Debug)]
pub struct MqttPacket {
    buffer: Bytes,
}

impl MqttPacket {
    pub(crate) fn new(buffer: Bytes) -> Self {
        Self { buffer }
    }

    pub fn get_packet(&self) -> Result<MPacket<'_>, MqttError> {
        Ok(parse_packet(&self.buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use crate::packet_storage::PacketStorage;

    assert_impl_all!(PacketStorage: Send, Sync);
}
