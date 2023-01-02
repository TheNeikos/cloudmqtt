//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
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
    sync::{atomic::AtomicU16, Arc},
};

use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::{mapref::entry::Entry, DashMap};
use mqtt_format::v3::{identifier::MPacketIdentifier, packet::MPublish, qos::MQualityOfService};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace};

use crate::error::MqttError;

use super::{message::MqttMessage, ClientConnection};

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
enum StoredMessage {
    /// A QoS 1 or 2 message that has been sent, it is saved here as it may need to be resent on
    /// reconnect
    Sent(MqttMessage),
    /// A QoS 2 message that has been acknowledged but not yet released.
    Acknowledged,
}

#[derive(Debug, Default)]
pub struct ClientState {
    connection_token: ArcSwap<CancellationToken>,
    conn: Arc<ArcSwapOption<ClientConnection>>,
    current_id: AtomicU16,
    stored_messages: DashMap<MPacketIdentifier, StoredMessage>,
}

impl ClientState {
    pub(crate) async fn set_new_connection(&self, conn: Arc<ClientConnection>) {
        let token = self
            .connection_token
            .swap(Arc::new(CancellationToken::new()));
        token.cancel();

        let old_conn = self.conn.swap(Some(conn));
        if let Some(old_conn) = old_conn {
            tokio::spawn(async move {
                let mut lock = old_conn.writer.lock().await;

                let _ = lock.shutdown().await;
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

        for stored_message in &self.stored_messages {
            let conn = self.conn.clone();
            let token = self.connection_token.load().clone();
            let msg = match stored_message.value() {
                StoredMessage::Sent(msg) => msg.clone(),
                StoredMessage::Acknowledged => continue,
            };
            let key = *stored_message.key();
            tokio::spawn(Self::with_connection(token, conn, move |conn| async move {
                let packet = MPublish {
                    dup: false,
                    qos: msg.qos(),
                    retain: msg.retain(),
                    topic_name: mqtt_format::v3::strings::MString { value: msg.topic() },
                    id: Some(key),
                    payload: msg.payload(),
                };
                let mut conn = conn.writer.lock().await;
                crate::write_packet(&mut *conn, packet).await?;
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

    pub async fn send_message(&self, msg: MqttMessage) {
        let entry = self
            .get_next_packet_entry()
            .expect("Exhausted available identifier slots");

        match msg.qos() {
            MQualityOfService::AtMostOnce => {
                let conn = self.conn.clone();
                let token = self.connection_token.load().clone();
                tokio::spawn(Self::with_connection(token, conn, move |conn| async move {
                    let msg = msg;
                    let packet = MPublish {
                        dup: false,
                        qos: msg.qos(),
                        retain: msg.retain(),
                        topic_name: mqtt_format::v3::strings::MString { value: msg.topic() },
                        id: None,
                        payload: msg.payload(),
                    };
                    let mut conn = conn.writer.lock().await;
                    crate::write_packet(&mut *conn, packet).await?;
                    Ok(())
                }));
            }
            MQualityOfService::AtLeastOnce | MQualityOfService::ExactlyOnce => {
                let conn = self.conn.clone();
                let token = self.connection_token.load().clone();
                let key = *entry.key();
                entry.or_insert(StoredMessage::Sent(msg.clone()));

                tokio::spawn(Self::with_connection(token, conn, move |conn| async move {
                    let packet = MPublish {
                        dup: false,
                        qos: msg.qos(),
                        retain: msg.retain(),
                        topic_name: mqtt_format::v3::strings::MString { value: msg.topic() },
                        id: Some(key),
                        payload: msg.payload(),
                    };
                    let mut conn = conn.writer.lock().await;
                    crate::write_packet(&mut *conn, packet).await?;

                    Ok(())
                }));
            }
        }
    }

    pub fn receive_puback(&self, id: MPacketIdentifier) -> Result<(), ()> {
        if let Some(msg) = self.stored_messages.get(&id) {
            match msg.value() {
                StoredMessage::Sent(msg) => {
                    if msg.qos() != MQualityOfService::AtLeastOnce {
                        debug!(
                            ?id,
                            "Received a PUBACK for a non-QoS 1 message (QoS was {:?})",
                            msg.qos()
                        );
                        return Err(());
                    }
                }
                StoredMessage::Acknowledged => {
                    debug!(
                        ?id,
                        "Received a PUBACK for an already acknowledged QoS 2 message"
                    );
                    return Err(());
                }
            }

            drop(msg);
            self.stored_messages.remove(&id);
            trace!(?id, "Removed QoS 1 message after acknowledging it");
        } else {
            debug!(?id, "Received a PUBACK for a nonexistent message");
            return Err(());
        }

        Ok(())
    }
    pub fn receive_pubrec(&self, id: MPacketIdentifier) -> Result<(), ()> {
        if let Some(msg) = self.stored_messages.get(&id) {
            match msg.value() {
                StoredMessage::Sent(msg) => {
                    if msg.qos() != MQualityOfService::ExactlyOnce {
                        debug!(
                            ?id,
                            "Received a PUBREC for a non-QoS 1 message (QoS was {:?})",
                            msg.qos()
                        );
                        return Err(());
                    }
                }
                StoredMessage::Acknowledged => {
                    debug!(
                        ?id,
                        "Received a PUBREC for an already acknowledged QoS 2 message"
                    );
                    return Err(());
                }
            }

            drop(msg);
            self.stored_messages.insert(id, StoredMessage::Acknowledged);
            trace!(?id, "Acknowledged QoS 2 message, storing identifier");
        } else {
            debug!(?id, "Received a PUBREC for a nonexistent message");
            return Err(());
        }

        Ok(())
    }

    pub fn receive_pubcomp(&self, id: MPacketIdentifier) -> Result<(), ()> {
        if let Some(msg) = self.stored_messages.get(&id) {
            match msg.value() {
                StoredMessage::Sent(_msg) => {
                    debug!(
                        ?id,
                        "Received a PUBCOMP for an already acknowledged QoS 2 message"
                    );
                    return Err(());
                }
                StoredMessage::Acknowledged => {
                    // We good
                }
            }

            drop(msg);
            self.stored_messages.remove(&id);
            trace!(?id, "Removed QoS 2 message after completing it");
        } else {
            debug!(?id, "Received a PUBCOMP for a nonexistent message");
            return Err(());
        }

        Ok(())
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
