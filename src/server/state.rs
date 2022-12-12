//! The state of a single client as defined by their ClientID
//!
//! # Architecture
//!
//! The guiding principle of this module is _no data loss_. The MQTT spec makes this fairly
//! straightforward to achieve, and such the major architectural parts should follow that guidance.
//!
//! As such, the client is split up into multiple parts:
//!
//! - A connection that is potentially existent
//! - A list of stored messages and the next useable id
//!
//! The process is as follows:
//!
//! - When a new message comes in:
//!   - Check if it is QoS == 0, in which case you send it on if there is a connection, otherwise
//!   you save or discard it
//!   - QoS must then be 1 or 2, as such we first save it before sending it onwards if there is a
//!   connection
//! - When a PUBACK/PUBREC/PUBCOMP comes in, we replace the given message in the map with the
//! answer and send it onwards (or we are done and simply delete the entry)
//! - If a new connection comes in the old one gets disconnected
//! - If a new connection comes in then all messages that are currently stored are sent over the
//! new connection, starting from the `current_id` field.
//!

use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{atomic::AtomicU16, Arc},
};

use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::{mapref::entry::Entry, DashMap};
use mqtt_format::v3::{
    identifier::MPacketIdentifier,
    packet::{MPacket, MPublish},
    qos::MQualityOfService,
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{error::MqttError, mqtt_stream::MqttStream, MqttPacket, MqttSubPacket};

use super::ClientConnection;

/// A Message that is unacknowledged, at what state depends on the current `packet`
///
/// This is generally only messages with a QoS > 0, but may also include those if configured
/// Behavior:
///
/// - If a `PUBLISH`, then the message has not been acknowledged by the client for any QoS > 0
///   - If it is QoS == 0, then it will be discarded once the stream has been flushed with that
///     packet
///   - If it is QoS == 1, then it will be discarded once the client sent a PUBACK response
///   - If it is QoS == 2, then the packet will be replaced (and the old one replaced) with the
///     `PUBREL` message in its place
/// - If a `PUBREL`, then we were in QoS 2, so we wait for a PUBCOMP to arrive with given
///   packet identifier after which we discard this state
#[derive(Debug)]
struct StoredMessage {
    packet: MqttPacket,
}

#[derive(Debug, Default)]
pub struct ClientState {
    connection_token: ArcSwap<CancellationToken>,
    conn: Arc<ArcSwapOption<ClientConnection>>,
    current_id: AtomicU16,
    stored_messages: DashMap<MPacketIdentifier, StoredMessage>,
}

impl ClientState {
    pub async fn set_new_connection(&self, conn: Arc<ClientConnection>) {
        let token = self
            .connection_token
            .swap(Arc::new(CancellationToken::new()));
        token.cancel();

        let old_conn = self.conn.swap(Some(conn));
        if let Some(old_conn) = old_conn {
            tokio::spawn(async move {
                let mut lock = old_conn.writer.lock().await;

                lock.shutdown().await;
            });
        }

        let mut messages: VecDeque<_> = self.stored_messages.iter().collect();
        let last_message_id = {
            let current_id = self.current_id.load(std::sync::atomic::Ordering::SeqCst);
            messages
                .iter()
                .enumerate()
                .skip_while(|(_, msg)| msg.key().0 < current_id)
                .map(|(idx, _)| idx)
                .next()
        };
        if let Some(last_message_id) = last_message_id {
            messages.rotate_left(last_message_id);
        }

        let messages: Vec<MqttPacket> = messages
            .into_iter()
            .map(|entry| MqttPacket::clone(&entry.value().packet))
            .collect();

        for message in messages {
            let token = self.connection_token.load().clone();
            let conn = self.conn.clone();
            tokio::spawn(Self::with_connection(token, conn, |conn| async move {
                let mut conn = conn.writer.lock().await;
                crate::write_packet(&mut *conn, *message.get_packet()).await?;
                Ok(())
            }));
        }
    }

    async fn with_connection<'a, F, D>(
        token: Arc<CancellationToken>,
        conn: Arc<ArcSwapOption<ClientConnection>>,
        fun: F,
    ) where
        F: FnOnce(Arc<ClientConnection>) -> D,
        F: 'a,
        D: std::future::Future<Output = Result<(), MqttError>> + Send + 'a,
    {
        let conn = conn.load_full();
        if let Some(conn) = conn {
            {
                tokio::select! {
                    _ = fun(conn) => {}
                    _ = token.cancelled() => {}
                }
            }
        }
    }

    pub async fn send_message(
        &self,
        minimal_qos: MQualityOfService,
        due_to_subscription: bool,
        msg: MqttSubPacket<MPublish<'static>>,
    ) {
        let entry = self
            .get_next_packet_entry()
            .expect("Exhausted available identifier slots");

        let packet = msg.get_packet();

        let actual_qos = minimal_qos.min(packet.qos);

        if actual_qos == MQualityOfService::AtMostOnce {
            let conn = self.conn.clone();
            let token = self.connection_token.load().clone();
            let entry = *entry.key();
            tokio::spawn(Self::with_connection(token, conn, move |conn| async move {
                let msg = msg;
                let packet = msg.get_packet();
                let packet = MPublish {
                    dup: false,
                    qos: actual_qos,
                    retain: due_to_subscription,
                    topic_name: packet.topic_name,
                    id: if actual_qos == MQualityOfService::AtMostOnce {
                        None
                    } else {
                        Some(entry)
                    },
                    payload: packet.payload,
                };
                let mut conn = conn.writer.lock().await;
                crate::write_packet(&mut *conn, packet).await?;
                Ok(())
            }));
        }
    }

    fn get_next_packet_entry(&self) -> Option<Entry<MPacketIdentifier, StoredMessage>> {
        if self.stored_messages.len() == u16::MAX as usize {
            debug!("We are storing u16::MAX messages, cannot allocate more messages");
            return None;
        }

        let next_id = match self
            .current_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        {
            // 0 is not a valid identifier
            0 => self
                .current_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            other => other,
        };

        let entry = self.stored_messages.entry(MPacketIdentifier(next_id));

        if matches!(entry, Entry::Occupied(_)) {
            self.get_next_packet_entry()
        } else {
            Some(entry)
        }
    }
}
