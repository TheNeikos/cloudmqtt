//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::{collections::VecDeque, sync::Arc};

use dashmap::DashMap;
use mqtt_format::v3::{
    connect_return::MConnectReturnCode, packet::MPacket, qos::MQualityOfService, strings::MString,
    will::MLastWill,
};
use tokio::{
    io::{AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf},
    net::{TcpListener, ToSocketAddrs},
    sync::{broadcast, Mutex},
};
use tracing::{debug, error, info, trace};

use crate::{error::MqttError, mqtt_stream::MqttStream, MqttPacket, PacketIOError};

#[derive(Debug, Clone)]
struct MqttMessage {
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
}

pub struct MqttServer {
    clients: DashMap<ClientId, ClientSession>,
    client_source: ClientSource,
    published_packets: broadcast::Sender<MqttMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClientId(String);

impl<'message> TryFrom<MString<'message>> for ClientId {
    type Error = ClientError;

    fn try_from(ms: MString<'message>) -> Result<Self, Self::Error> {
        Ok(ClientId(ms.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
enum ClientError {
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
        let (published_packets, _) = broadcast::channel(100);

        Ok(MqttServer {
            clients: DashMap::new(),
            client_source: ClientSource::UnsecuredTcp(bind),
            published_packets,
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
        } = packet.get_packet()?
        {
            let client_id = ClientId::try_from(client_id)?;

            let session_present = if clean_session {
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

            let (reader, writer) = tokio::io::split(client);

            let client_connection = Arc::new(ClientConnection {
                reader: Mutex::new(reader),
                writer: Mutex::new(writer),
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

            let published_packets = self.published_packets.clone();
            let mut published_packets_rec = published_packets.subscribe();

            let published_conn = client_connection.clone();
            let published_client_id = client_id.clone();
            tokio::spawn(async move {
                let client_id = published_client_id;
                let published_conn = published_conn;

                loop {
                    match published_packets_rec.recv().await {
                        Ok(packet) => {
                            if packet.author_id == client_id {
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

                            let mut writer = published_conn.writer.lock().await;
                            crate::write_packet(&mut *writer, packet).await.unwrap();
                            // 1. Check if subscription matches
                            // 2. If Qos == 0 -> Send into writer
                            // 3. If QoS == 1 -> Send into writer && Store Message waiting for Puback
                            // 4. If QoS == 2 -> Send into writer && Store Message waiting for PubRec
                        }
                        Err(_) => todo!(),
                    }
                }
            });

            tokio::spawn(async move {
                let client_id = client_id;
                let client_connection = client_connection;
                let mut reader = client_connection.reader.lock().await;

                loop {
                    let packet = match crate::read_one_packet(&mut *reader).await {
                        Ok(packet) => packet,
                        Err(e) => {
                            debug!("Could not read the next client packet: {e}");
                            break;
                        }
                    };

                    match packet.get_packet()? {
                        MPacket::Publish {
                            dup,
                            qos,
                            retain,
                            topic_name,
                            id,
                            payload,
                        } => {
                            let message = MqttMessage {
                                author_id: client_id.clone(),
                                topic: topic_name.to_string(),
                                retain,
                                qos,
                                payload: payload.to_vec(),
                            };

                            if let Err(_) = published_packets.send(message) {
                                error!("Could not broadcast message");
                                todo!()
                            }

                            if qos == MQualityOfService::AtLeastOnce {
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
                            break;
                        }
                        packet => info!("Received packet: {packet:?}"),
                    }
                }

                if let Some(will) = last_will {
                    debug!(?will, "Sending out will");
                    let _ = published_packets.send(will);
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
