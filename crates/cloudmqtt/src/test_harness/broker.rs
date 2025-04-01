//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use dashmap::DashMap;
use futures::SinkExt;
use futures::StreamExt;
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
        let connection = Framed::new(connection, crate::codec::MqttPacketCodec);

        let state = ConnectionState { connection };

        self.connections.insert(client_name, state);
        Ok(())
    }

    pub(crate) async fn send(
        &mut self,
        client_name: &str,
        packet: mqtt_format::v5::packets::MqttPacket<'_>,
    ) -> Result<(), TestHarnessError> {
        let Some(mut r) = self.connections.get_mut(client_name) else {
            return Err(TestHarnessError::ClientNotFound(client_name.to_string()));
        };

        r.value_mut()
            .connection
            .send(packet)
            .await
            .map_err(TestHarnessError::Codec)
    }

    pub(crate) async fn recv_packet(
        &self,
        client_name: &str,
    ) -> Result<crate::codec::MqttPacket, TestHarnessError> {
        let packet = {
            let Some(mut r) = self.connections.get_mut(client_name) else {
                tracing::warn!(name = ?client_name, "Failed to find client");
                return Err(TestHarnessError::ClientNotFound(client_name.to_string()));
            };
            tracing::debug!(name = ?client_name, "Client found, fetching next packet");

            let Some(next) = r.value_mut().connection.next().await else {
                tracing::warn!(name = ?client_name, "Stream to client closed");
                return Err(TestHarnessError::StreamClosed(client_name.to_string()));
            };

            next
        };

        packet.map_err(TestHarnessError::Codec)
    }

    pub(crate) async fn wait_received(
        &self,
        client_name: &str,
        expected_packet: mqtt_format::v5::packets::MqttPacket<'_>,
    ) -> Result<(), TestHarnessError> {
        let packet = self.recv_packet(client_name).await?;

        if *packet.get_packet() == expected_packet {
            tracing::trace!("Packet as expected");
            Ok(())
        } else {
            tracing::warn!(expected = ?expected_packet, received = ?packet, "Packet not as expected");
            Err(TestHarnessError::PacketNotExpected {
                got: Box::new(packet),
            })
        }
    }
}

struct ConnectionState {
    connection: Framed<tokio::io::DuplexStream, crate::codec::MqttPacketCodec>,
}
