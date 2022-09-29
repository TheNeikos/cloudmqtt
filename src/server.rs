//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::{collections::VecDeque, sync::Arc, time::Duration};

use dashmap::DashMap;
use mqtt_format::v3::{
    connect_return::MConnectReturnCode, packet::MPacket, qos::MQualityOfService, strings::MString,
    will::MLastWill,
};
use tokio::{
    io::{AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf},
    net::{TcpListener, ToSocketAddrs},
    sync::{Mutex},
};
use tracing::{debug, error, info, trace};

use crate::{
    error::MqttError,
    mqtt_stream::MqttStream,
    topics::{ClientInformation, SubscriptionManager},
    MqttPacket, PacketIOError,
};

#[derive(Debug, Clone)]
pub struct MqttMessage {
    author_id: Arc<ClientId>,
    topic: String,
    payload: Vec<u8>,
    retain: bool,
    qos: MQualityOfService,
}

impl MqttMessage {
    fn from_last_will(will: &MLastWill, author_id: Arc<ClientId>) -> MqttMessage {
        MqttMessage {
            topic: will.topic.to_string(),
            payload: will.payload.to_vec(),
            qos: will.qos,
            retain: will.retain,
            author_id,
        }
    }

    pub fn qos(&self) -> MQualityOfService {
        self.qos
    }

    pub fn retain(&self) -> bool {
        self.retain
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }

    pub fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    pub fn author_id(&self) -> &ClientId {
        self.author_id.as_ref()
    }
}

pub struct MqttServer {
    clients: DashMap<ClientId, ClientSession>,
    client_source: ClientSource,
    subscription_manager: SubscriptionManager,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientId(String);

impl<'message> TryFrom<MString<'message>> for ClientId {
    type Error = ClientError;

    fn try_from(ms: MString<'message>) -> Result<Self, Self::Error> {
        Ok(ClientId(ms.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("An error occured during the handling of a packet")]
    Packet(#[from] PacketIOError),
}

#[derive(Debug)]
struct ClientConnection {
    reader: Mutex<ReadHalf<MqttStream>>,
    writer: Mutex<WriteHalf<MqttStream>>,
}

#[derive(Debug, Default)]
struct ClientSession {
    conn: Option<Arc<ClientConnection>>,
    unacknowledged_messages: VecDeque<MqttPacket>,
}

#[derive(Debug)]
enum ClientSource {
    UnsecuredTcp(TcpListener),
    Duplex(tokio::sync::mpsc::Receiver<DuplexStream>),
}

impl ClientSource {
    async fn accept(&mut self) -> Result<MqttStream, MqttError> {
        Ok({
            match self {
                ClientSource::UnsecuredTcp(listener) => listener
                    .accept()
                    .await
                    .map(|tpl| tpl.0)
                    .map(MqttStream::UnsecuredTcp)?,
                ClientSource::Duplex(recv) => recv
                    .recv()
                    .await
                    .map(MqttStream::MemoryDuplex)
                    .ok_or(MqttError::DuplexSourceClosed)?,
            }
        })
    }
}

impl MqttServer {
    pub async fn serve_v3_unsecured_tcp<Addr: ToSocketAddrs>(
        addr: Addr,
    ) -> Result<Self, MqttError> {
        let bind = TcpListener::bind(addr).await?;

        Ok(MqttServer {
            clients: DashMap::new(),
            client_source: ClientSource::UnsecuredTcp(bind),
            subscription_manager: SubscriptionManager::new(),
        })
    }

    pub async fn accept_new_clients(&mut self) -> Result<(), MqttError> {
        loop {
            let client = self.client_source.accept().await?;
            if let Err(client_error) = self.accept_client(client).await {
                tracing::error!("Client error: {}", client_error)
            }
        }
    }

    async fn accept_client(&mut self, mut client: MqttStream) -> Result<(), ClientError> {
        let packet = crate::read_one_packet(&mut client).await?;

        if let MPacket::Connect {
            client_id,
            clean_session,
            protocol_name: _,
            protocol_level: _,
            will,
            username: _,
            password: _,
            keep_alive,
        } = packet.get_packet()
        {
            let client_id = ClientId::try_from(*client_id)?;

            let session_present = if *clean_session {
                let _ = self.clients.remove(&client_id);
                false
            } else {
                self.clients.contains_key(&client_id)
            };

            let conn_ack = MPacket::Connack {
                session_present,
                connect_return_code: MConnectReturnCode::Accepted,
            };

            crate::write_packet(&mut client, conn_ack).await?;

            let (client_reader, client_writer) = tokio::io::split(client);

            let client_connection = Arc::new(ClientConnection {
                reader: Mutex::new(client_reader),
                writer: Mutex::new(client_writer),
            });

            {
                let mut state = self
                    .clients
                    .entry(client_id.clone())
                    .or_insert_with(ClientSession::default);
                state.conn = Some(client_connection.clone());
            }

            let client_id = Arc::new(client_id);

            let mut last_will: Option<MqttMessage> = will
                .as_ref()
                .map(|will| MqttMessage::from_last_will(will, client_id.clone()));

            let published_packets = self.subscription_manager.clone();
            let (published_packets_send, mut published_packets_rec) =
                tokio::sync::mpsc::unbounded_channel::<MqttMessage>();

            let publisher_conn = client_connection.clone();
            let publisher_client_id = client_id.clone();
            tokio::spawn(async move {
                loop {
                    match published_packets_rec.recv().await {
                        Some(packet) => {
                            if packet.author_id == publisher_client_id {
                                trace!(?packet, "Skipping sending message to oneself");
                                continue;
                            }

                            let packet = MPacket::Publish {
                                dup: false,
                                qos: MQualityOfService::AtMostOnce,
                                retain: packet.retain,
                                topic_name: MString {
                                    value: &packet.topic,
                                },
                                id: None,
                                payload: &packet.payload,
                            };

                            let mut writer = publisher_conn.writer.lock().await;
                            crate::write_packet(&mut *writer, packet).await.unwrap();
                            // 1. Check if subscription matches
                            // 2. If Qos == 0 -> Send into writer
                            // 3. If QoS == 1 -> Send into writer && Store Message waiting for Puback
                            // 4. If QoS == 2 -> Send into writer && Store Message waiting for PubRec
                        }
                        None => {
                            debug!(?publisher_client_id, "No more senders, stopping sending cycle");
                        }
                    }
                }
            });

            let keep_alive = *keep_alive;
            let subscription_manager = self.subscription_manager.clone();

            tokio::spawn(async move {
                let client_id = client_id;
                let client_connection = client_connection;
                let mut reader = client_connection.reader.lock().await;
                let keep_alive_duration = Duration::from_secs((keep_alive as u64 * 150) / 100);
                let subscription_manager = subscription_manager;

                loop {
                    let packet = tokio::select! {
                        packet = crate::read_one_packet(&mut *reader) => {
                            match packet {
                                Ok(packet) => packet,
                                Err(e) => {
                                    debug!("Could not read the next client packet: {e}");
                                    break;
                                }
                            }
                        },
                        _timeout = tokio::time::sleep(keep_alive_duration) => {
                            debug!("Client timed out");
                            break;
                        }
                    };

                    match packet.get_packet() {
                        MPacket::Publish {
                            dup: _,
                            qos,
                            retain,
                            topic_name,
                            id,
                            payload,
                        } => {
                            let message = MqttMessage {
                                author_id: client_id.clone(),
                                topic: topic_name.to_string(),
                                retain: *retain,
                                qos: *qos,
                                payload: payload.to_vec(),
                            };

                            subscription_manager.route_message(message).await;

                            if *qos == MQualityOfService::AtLeastOnce {
                                let packet = MPacket::Puback { id: id.unwrap() };
                                let mut writer = client_connection.writer.lock().await;
                                crate::write_packet(&mut *writer, packet).await?;
                            }

                            // tokio::spawn(publish_state_machine -> {
                            //     if qos == 0  {
                            //         -> Send message to other clients on topic
                            //     }
                            //     if qos == 1 {
                            //         -> Send PUBACK back
                            //         -> Send message to other clients on topic with QOS 1
                            //             published_packets.send(message)
                            //     }
                            //     if qos == 2 {
                            //         -> Store Packet Identifier
                            //         -> Send message to other clients on topic with QOS 2
                            //         -> Send PUBREC
                            //         -> Save in MessageStore with latest state = PUBREC
                            //     }
                            // })
                        }
                        MPacket::Pubrel { .. } => {
                            // -> Check if MessageStore contains state PUBREC with packet id
                        }
                        MPacket::Pubrec { .. } => {
                            // -> Discard message
                            // -> Store PUBREC received
                            // -> Send PUBREL
                        }
                        MPacket::Pubcomp { .. } => {
                            // -> Discard PUBREC
                        }
                        MPacket::Disconnect => {
                            last_will.take();
                            debug!("Client disconnected gracefully");
                            break;
                        }
                        MPacket::Subscribe { id: _, subscriptions } => {
                            subscription_manager
                                .subscribe(
                                    Arc::new(ClientInformation {
                                        client_id: client_id.clone(),
                                        client_sender: published_packets_send.clone(),
                                    }),
                                    *subscriptions,
                                )
                                .await;
                        }
                        packet => info!("Received packet: {packet:?}"),
                    }
                }

                if let Some(will) = last_will {
                    debug!(?will, "Sending out will");
                    let _ = published_packets.route_message(will);
                }

                if let Err(e) = client_connection.writer.lock().await.shutdown().await {
                    debug!("Client could not shut down cleanly: {e}");
                }

                Ok::<(), ClientError>(())
            });
        } else {
            // Disconnect and don't worry about errors
            if let Err(e) = client.shutdown().await {
                debug!("Client could not shut down cleanly: {e}");
            }
        }

        Ok(())
    }
}
