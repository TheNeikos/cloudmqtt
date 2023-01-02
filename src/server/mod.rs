//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! MQTTServer and internals
//!
//! Server Architecture
//! ===================
//!
//! The server consists of multiple parts:
//!
//! - An [`MQTTServer`], the main part of the whole and the user-visible part
//! - The [`SubscriptionManager`] which maintains subscription state
//!
//! Per [MQTT Spec] in 3.1.2.4 the server has to keep the following state:
//!
//! - Whether a session exists -> [`ClientSession`]
//! - The clients subscriptions -> [`SubscriptionManager`]
//!
//! This implementation utilizes "Method B" for QoS 2 protocol flow, as explained in Figure 4.3 of
//! the [MQTT Spec]. This minimizes data being held in the application.
//!
//! - QoS 1 & 2 messages which have been relayed to the client, but not yet acknowledged
//! - QoS 0 & 1 & 2 messages pending transmission
//! - QoS 2 messages which have been received from the client, but have not been acknowledged
//!
//!
//! [MQTT Spec]: http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html
#![deny(missing_docs)]

mod message;
mod state;
mod subscriptions;

use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use mqtt_format::v3::{
    connect_return::MConnectReturnCode,
    packet::{
        MConnack, MConnect, MDisconnect, MPacket, MPingreq, MPingresp, MPuback, MPubcomp, MPublish,
        MPubrec, MPubrel, MSubscribe,
    },
    qos::MQualityOfService,
    strings::MString,
    will::MLastWill,
};
use tokio::{
    io::{AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf},
    net::{TcpListener, ToSocketAddrs},
    sync::Mutex,
};
use tracing::{debug, error, info, trace};

use crate::{error::MqttError, mqtt_stream::MqttStream, PacketIOError};
use subscriptions::{ClientInformation, SubscriptionManager};

use self::{message::MqttMessage, state::ClientState};

/// The unique id (per server) of a connecting client
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientId(String);

impl ClientId {
    #[allow(dead_code)]
    pub(crate) fn new(id: String) -> Self {
        ClientId(id)
    }
}

impl<'message> TryFrom<MString<'message>> for ClientId {
    type Error = ClientError;

    fn try_from(ms: MString<'message>) -> Result<Self, Self::Error> {
        Ok(ClientId(ms.to_string()))
    }
}

/// An error that occurred while communicating with a client
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// An error occurred during the sending/receiving of a packet
    #[error("An error occured during the handling of a packet")]
    Packet(#[from] PacketIOError),
}

#[derive(Debug)]
pub(crate) struct ClientConnection {
    reader: Mutex<ReadHalf<MqttStream>>,
    writer: Mutex<WriteHalf<MqttStream>>,
}

#[derive(Debug)]
enum ClientSource {
    UnsecuredTcp(TcpListener),
    #[allow(dead_code)]
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

/// A complete MQTT Server
///
/// This server should be seen as a toolkit to integrate with your application.
///
/// To use it, you first need to get a new instance of it, check out any of the `serve_*` methods
/// that create a new instance.
/// Then, you need to start listening for new connections in a long lived future.
///
/// Check out the server example for a working version.
///
pub struct MqttServer {
    clients: Arc<DashMap<ClientId, ClientState>>,
    client_source: Mutex<ClientSource>,
    subscription_manager: SubscriptionManager,
}

impl MqttServer {
    /// Create a new MQTT server listening on the given `SocketAddr`
    pub async fn serve_v3_unsecured_tcp<Addr: ToSocketAddrs>(
        addr: Addr,
    ) -> Result<Self, MqttError> {
        let bind = TcpListener::bind(addr).await?;

        Ok(MqttServer {
            clients: Arc::new(DashMap::new()),
            client_source: Mutex::new(ClientSource::UnsecuredTcp(bind)),
            subscription_manager: SubscriptionManager::new(),
        })
    }

    /// Start accepting new clients connecting to the server
    pub async fn accept_new_clients(self: Arc<Self>) -> Result<(), MqttError> {
        let mut client_source = self
            .client_source
            .try_lock()
            .map_err(|_| MqttError::AlreadyListening)?;

        loop {
            let client = client_source.accept().await?;
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(client_error) = server.accept_client(client).await {
                    tracing::error!("Client error: {}", client_error)
                }
            });
        }
    }

    /// Accept a new client connected through the `client` stream
    ///
    /// This does multiple things:
    ///
    /// - It checks whether a client with that given ID exists
    ///     - If yes, then that session is replaced when clean_session = true
    ///
    async fn accept_client(&self, mut client: MqttStream) -> Result<(), ClientError> {
        async fn send_connack(
            session_present: bool,
            connect_return_code: MConnectReturnCode,
            client: &mut MqttStream,
        ) -> Result<(), ClientError> {
            let conn_ack = MConnack {
                session_present,
                connect_return_code,
            };

            crate::write_packet(client, conn_ack).await?;

            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        async fn connect_client<'message>(
            server: &MqttServer,
            mut client: MqttStream,
            _protocol_name: MString<'message>,
            _protocol_level: u8,
            clean_session: bool,
            will: Option<MLastWill<'message>>,
            _username: Option<MString<'message>>,
            _password: Option<&'message [u8]>,
            keep_alive: u16,
            client_id: MString<'message>,
        ) -> Result<(), ClientError> {
            let empty_client_id = client_id.is_empty();

            let client_id = ClientId::try_from(client_id)?;

            // Check MQTT-3.1.37: "If the Client supplies a zero-byte ClientId,
            // the Client MUST also set CleanSession to 1"
            if empty_client_id && !clean_session {
                // Send a CONNACK with code 0x02/IdentifierRejected
                if let Err(e) =
                    send_connack(false, MConnectReturnCode::IdentifierRejected, &mut client).await
                {
                    debug!("Client could not shut down cleanly: {e}");
                }

                return Err(ClientError::Packet(PacketIOError::InvalidParsedPacket));
            }

            let session_present = if clean_session {
                let _ = server.clients.remove(&client_id);
                false
            } else {
                server.clients.contains_key(&client_id)
            };

            send_connack(session_present, MConnectReturnCode::Accepted, &mut client).await?;
            debug!(?client_id, "Accepted new connection");

            let (client_reader, client_writer) = tokio::io::split(client);

            let client_connection = Arc::new(ClientConnection {
                reader: Mutex::new(client_reader),
                writer: Mutex::new(client_writer),
            });

            {
                let client_state = server
                    .clients
                    .entry(client_id.clone())
                    .or_insert_with(ClientState::default);
                client_state
                    .set_new_connection(client_connection.clone())
                    .await;
            }

            let client_id = Arc::new(client_id);

            let mut last_will: Option<MqttMessage> = will
                .as_ref()
                .map(|will| MqttMessage::from_last_will(will, client_id.clone()));

            let published_packets = server.subscription_manager.clone();
            let (published_packets_send, mut published_packets_rec) =
                tokio::sync::mpsc::unbounded_channel::<MqttMessage>();

            let send_loop = {
                let publisher_client_id = client_id.clone();
                let clients = server.clients.clone();
                tokio::spawn(async move {
                    loop {
                        match published_packets_rec.recv().await {
                            Some(packet) => {
                                if packet.author_id() == &*publisher_client_id {
                                    trace!(?packet, "Skipping sending message to oneself");
                                    continue;
                                }

                                let Some(client_state) = clients.get(&publisher_client_id) else {
                                    debug!(?publisher_client_id, "Associated state no longer exists");
                                    break;
                                };

                                client_state.send_message(packet).await;
                            }
                            None => {
                                debug!(
                                    ?publisher_client_id,
                                    "No more senders, stopping sending cycle"
                                );
                                break;
                            }
                        }
                    }
                })
            };

            let read_loop = {
                let keep_alive = keep_alive;
                let subscription_manager = server.subscription_manager.clone();
                let client_id = client_id.clone();
                let clients = server.clients.clone();

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
                            MPacket::Publish(MPublish {
                                dup: _,
                                qos,
                                retain,
                                topic_name,
                                id,
                                payload,
                            }) => {
                                let message = MqttMessage::new(
                                    client_id.clone(),
                                    payload.to_vec(),
                                    topic_name.to_string(),
                                    *retain,
                                    *qos,
                                );

                                subscription_manager.route_message(message).await;

                                // Handle QoS 1/AtLeastOnce response
                                if *qos == MQualityOfService::AtLeastOnce {
                                    let packet = MPuback { id: id.unwrap() };
                                    let mut writer = client_connection.writer.lock().await;
                                    crate::write_packet(&mut *writer, packet).await?;
                                }

                                if *qos == MQualityOfService::ExactlyOnce {
                                    let Some(client_state) = clients.get(&client_id) else {
                                        debug!(?client_id, "Associated state no longer exists");
                                        break;
                                    };

                                    if let Err(_err) =
                                        client_state.save_qos_exactly_once(id.unwrap())
                                    {
                                        debug!("Encountered an error while handling a PUBACK");
                                        break;
                                    }

                                    let packet = MPubrec { id: id.unwrap() };
                                    let mut writer = client_connection.writer.lock().await;
                                    crate::write_packet(&mut *writer, packet).await?;
                                }
                            }
                            MPacket::Puback(ack @ MPuback { id }) => {
                                trace!(?client_id, ?ack, "Received puback");
                                let Some(client_state) = clients.get(&client_id) else {
                                    debug!(?client_id, "Associated state no longer exists");
                                    break;
                                };

                                if let Err(_err) = client_state.receive_puback(*id) {
                                    debug!("Encountered an error while handling a PUBACK");
                                    break;
                                }
                            }
                            MPacket::Pubrec(ack @ MPubrec { id }) => {
                                trace!(?client_id, ?ack, "Received pubrec");
                                let Some(client_state) = clients.get(&client_id) else {
                                    debug!(?client_id, "Associated state no longer exists");
                                    break;
                                };

                                if let Err(_err) = client_state.receive_pubrec(*id) {
                                    debug!("Encountered an error while handling a PUBACK");
                                    break;
                                }
                                trace!(?client_id, "Received PUBREC, responding with PUBREL");
                                let packet = MPubrel { id: *id };
                                let mut writer = client_connection.writer.lock().await;
                                crate::write_packet(&mut *writer, packet).await?;
                                trace!("Done responding to PUBREC with PUBREL");
                            }
                            MPacket::Pubrel(ack @ MPubrel { id }) => {
                                trace!(?client_id, ?ack, "Received pubrel");
                                let Some(client_state) = clients.get(&client_id) else {
                                    debug!(?client_id, "Associated state no longer exists");
                                    break;
                                };

                                if let Err(_err) = client_state.receive_pubrel(*id) {
                                    debug!("Encountered an error while handling a PUBREL");
                                    break;
                                }
                                let packet = MPubcomp { id: *id };
                                let mut writer = client_connection.writer.lock().await;
                                crate::write_packet(&mut *writer, packet).await?;
                                trace!("Done responding to PUBREL with PUBCOMP");
                            }
                            MPacket::Pubcomp(ack @ MPubcomp { id }) => {
                                trace!(?client_id, ?ack, "Received pubcomp");
                                let Some(client_state) = clients.get(&client_id) else {
                                    debug!(?client_id, "Associated state no longer exists");
                                    break;
                                };

                                if let Err(_err) = client_state.receive_pubcomp(*id) {
                                    debug!("Encountered an error while handling a PUBCOMP");
                                    break;
                                }
                            }
                            MPacket::Disconnect(MDisconnect) => {
                                last_will.take();
                                debug!("Client disconnected gracefully");
                                break;
                            }
                            MPacket::Subscribe(MSubscribe {
                                id: _,
                                subscriptions,
                            }) => {
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
                            MPacket::Pingreq(MPingreq) => {
                                trace!(
                                    ?client_id,
                                    "Received ping request, responding with ping response"
                                );
                                let packet = MPingresp;
                                let mut writer = client_connection.writer.lock().await;
                                crate::write_packet(&mut *writer, packet).await?;
                            }
                            packet => info!("Received packet: {packet:?}, not handling it"),
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
                })
            };

            let (send_err, read_err) = tokio::join!(send_loop, read_loop);
            match send_err {
                Ok(_) => (),
                Err(join_error) => error!(
                    "Send loop of client {} had an unexpected error: {join_error}",
                    &client_id.0
                ),
            }

            match read_err {
                Ok(_) => (),
                Err(join_error) => error!(
                    "Read loop of client {} had an unexpected error: {join_error}",
                    &client_id.0
                ),
            }

            Ok(())
        }

        trace!("Accepting new client");

        let packet = crate::read_one_packet(&mut client).await?;

        if let MPacket::Connect(MConnect {
            client_id,
            clean_session,
            protocol_name,
            protocol_level,
            will,
            username,
            password,
            keep_alive,
        }) = packet.get_packet()
        {
            trace!(?client_id, "Connecting client");
            connect_client(
                self,
                client,
                *protocol_name,
                *protocol_level,
                *clean_session,
                *will,
                *username,
                *password,
                *keep_alive,
                *client_id,
            )
            .await?;
        } else {
            // Disconnect and don't worry about errors
            if let Err(e) = client.shutdown().await {
                debug!("Client could not shut down cleanly: {e}");
            }
        }

        Ok(())
    }
}
