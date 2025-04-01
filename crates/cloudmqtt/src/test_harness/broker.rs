//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use dashmap::DashMap;
use futures::SinkExt;
use futures::StreamExt;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;

use super::error::TestHarnessError;

#[derive(Default)]
pub(crate) struct Broker {
    name: String,
    connections: DashMap<String, ConnectionState>,
}

impl std::fmt::Debug for Broker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Broker")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Broker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connections: DashMap::new(),
        }
    }

    pub(crate) async fn connect(
        &self,
        client_name: String,
        connection: tokio::io::DuplexStream,
    ) -> Result<(), TestHarnessError> {
        let mut connection = Framed::new(connection, crate::codec::MqttPacketCodec);

        let (packet_sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let (sender, mut packet_receiver) = tokio::sync::broadcast::channel(1);

        let received_packets = Arc::new(Mutex::new(Vec::new()));

        tokio::spawn({
            let rpacks = received_packets.clone();
            async move {
                loop {
                    let packet = packet_receiver.recv().await;
                    tracing::trace!(?packet, "Received packet on broker");
                    match packet {
                        Ok(p) => rpacks.lock().await.push(p),
                        Err(error) => tracing::warn!(?error),
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    packet = connection.next() => {
                        tracing::trace!(?packet, "Received next packet on connection");

                        let Some(packet) = packet else {
                            tracing::warn!("Stream closed");
                            break;
                        };

                        let packet = match packet {
                            Ok(p) => p,
                            Err(error) => {
                                tracing::warn!(?error);
                                break
                            }
                        };

                        sender.send(packet).unwrap();
                    }

                    packet_to_send = receiver.recv() => {
                        let Some(packet): Option<crate::codec::MqttPacket> = packet_to_send else {
                            tracing::warn!("Receiver closed");
                            break
                        };

                        tracing::trace!(?packet, "Received next packet to send to connection");
                        connection.send(packet.get_packet().clone()).await.unwrap()
                    }
                }
            }
        });

        let state = ConnectionState {
            packet_sender,
            received_packets,
        };

        self.connections.insert(client_name, state);
        Ok(())
    }

    pub(crate) async fn send(
        &mut self,
        client_name: &str,
        packet: mqtt_format::v5::packets::MqttPacket<'_>,
    ) -> Result<(), TestHarnessError> {
        let Some(sender) = self
            .connections
            .get(client_name)
            .map(|r| r.value().packet_sender.clone())
        else {
            return Err(TestHarnessError::ClientNotFound(client_name.to_string()));
        };

        tracing::debug!(?packet, "Sending out packet");
        sender
            .send(crate::codec::MqttPacket::new(packet))
            .await
            .map_err(|_| TestHarnessError::Channel)
    }

    pub(crate) async fn has_received_packet<F>(
        &self,
        client_name: &str,
        func: F,
    ) -> Result<bool, TestHarnessError>
    where
        F: Fn(&crate::codec::MqttPacket) -> bool,
    {
        let Some(packets) = self
            .connections
            .get(client_name)
            .map(|r| r.value().received_packets.clone())
        else {
            tracing::warn!(name = ?client_name, "Failed to find client");
            return Err(TestHarnessError::ClientNotFound(client_name.to_string()));
        };

        tracing::debug!(name = ?client_name, "Client found, fetching next packet");

        for p in packets.lock().await.iter() {
            if func(p) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

struct ConnectionState {
    packet_sender: tokio::sync::mpsc::Sender<crate::codec::MqttPacket>,

    received_packets: Arc<Mutex<Vec<crate::codec::MqttPacket>>>,
}
