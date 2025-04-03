//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;
use std::time::Instant;

use cloudmqtt_core::client::ExpectedAction;
use cloudmqtt_core::client::MqttClientFSM;
use cloudmqtt_core::client::MqttInstant;
use futures::SinkExt;
use futures::StreamExt;
use tokio::sync::Mutex;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

use crate::SendUsage;
use crate::codec::MqttPacket;
use crate::codec::MqttPacketCodec;
use crate::error::Error;

fn since(start: Instant) -> MqttInstant {
    MqttInstant::new(start.elapsed().as_secs())
}

pub struct CoreClient {
    incoming_sender: tokio::sync::mpsc::Sender<MqttPacket>,
    connection_state: Arc<Mutex<ConnectionState>>,
}

enum ConnectionState {
    Unconnected {
        client: MqttClientFSM,
    },

    Connected {
        sender: tokio::sync::mpsc::Sender<SendUsage>,
    },
}

impl CoreClient {
    pub fn new_and_connect<Read, Write>(
        reader: Read,
        writer: Write,
        incoming_sender: tokio::sync::mpsc::Sender<MqttPacket>,
    ) -> Self
    where
        Read: tokio::io::AsyncRead + Send + 'static,
        Write: tokio::io::AsyncWrite + Send + 'static,
    {
        let (sender, receiver): (tokio::sync::mpsc::Sender<SendUsage>, _) =
            tokio::sync::mpsc::channel(1);

        let start = Instant::now();

        let connection_state = Arc::new(Mutex::new(ConnectionState::Connected { sender }));

        let state_arc = connection_state.clone();
        let incoming_sender_clone = incoming_sender.clone();
        tokio::task::spawn(async move {
            let fsm = MqttClientFSM::default();

            let mut fsm =
                handle_connection(reader, writer, incoming_sender_clone, receiver, start, fsm)
                    .await;

            tracing::trace!("Connection lost. Telling FSM");
            fsm.connection_lost(since(start));

            tracing::trace!("Setting state to Unconnected");
            *state_arc.lock().await = ConnectionState::Unconnected { client: fsm };
        });

        Self {
            incoming_sender,
            connection_state,
        }
    }

    pub fn new(incoming_sender: tokio::sync::mpsc::Sender<MqttPacket>) -> Self {
        Self {
            incoming_sender,
            connection_state: Arc::new(Mutex::new(ConnectionState::Unconnected {
                client: MqttClientFSM::default(),
            })),
        }
    }

    pub async fn connect<Read, Write>(&self, reader: Read, writer: Write) -> Result<(), Error>
    where
        Read: tokio::io::AsyncRead + Send + 'static,
        Write: tokio::io::AsyncWrite + Send + 'static,
    {
        let (sender, receiver): (tokio::sync::mpsc::Sender<SendUsage>, _) =
            tokio::sync::mpsc::channel(1);

        let ConnectionState::Unconnected { client: fsm } =
            std::mem::replace(&mut *self.connection_state.lock().await, {
                ConnectionState::Connected { sender }
            })
        else {
            return Err(Error::AlreadyConnected);
        };

        let start = Instant::now();
        let state_arc = self.connection_state.clone();
        let incoming_sender_clone = self.incoming_sender.clone();
        tokio::task::spawn(async move {
            let mut fsm =
                handle_connection(reader, writer, incoming_sender_clone, receiver, start, fsm)
                    .await;

            tracing::trace!("Connection lost. Telling FSM");
            fsm.connection_lost(since(start));

            tracing::trace!("Setting state to Unconnected");
            *state_arc.lock().await = ConnectionState::Unconnected { client: fsm };
        });

        Ok(())
    }

    pub async fn publish(&self, packet: MqttPacket) -> Result<(), Error> {
        match *self.connection_state.lock().await {
            ConnectionState::Unconnected { .. } => {
                tracing::warn!("Tried to publish although not connected");
                Err(Error::NotConnected)
            }
            ConnectionState::Connected { ref sender } => {
                tracing::debug!("Sending out publish packet");
                sender
                    .send(SendUsage::Publish(packet))
                    .await
                    .map_err(|_| Error::TokioChannel)
            }
        }
    }

    pub async fn subscribe(&self, packet: MqttPacket) -> Result<(), Error> {
        match *self.connection_state.lock().await {
            ConnectionState::Unconnected { .. } => {
                tracing::warn!("Tried to subscribe although not connected");
                Err(Error::NotConnected)
            }
            ConnectionState::Connected { ref sender } => {
                tracing::debug!("Trying to subscribe");
                sender
                    .send(SendUsage::Subscribe(packet))
                    .await
                    .map_err(|_| Error::TokioChannel)
            }
        }
    }
}

async fn handle_connection<Read, Write>(
    reader: Read,
    writer: Write,
    incoming_sender: tokio::sync::mpsc::Sender<MqttPacket>,
    mut receiver: tokio::sync::mpsc::Receiver<SendUsage>,
    start: Instant,
    mut fsm: MqttClientFSM,
) -> MqttClientFSM
where
    Read: tokio::io::AsyncRead + Send + 'static,
    Write: tokio::io::AsyncWrite + Send + 'static,
{
    let writer = std::pin::pin!(writer);
    let reader = std::pin::pin!(reader);
    let mut writer = FramedWrite::new(writer, MqttPacketCodec);
    let mut reader = FramedRead::new(reader, MqttPacketCodec);

    tracing::trace!("Calling FSM to handle connect");
    let action = fsm.handle_connect(
        since(start),
        mqtt_format::v5::packets::connect::MConnect {
            client_identifier: "cloudmqtt-0",
            username: None,
            password: None,
            clean_start: true,
            will: None,
            properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
            keep_alive: 0,
        },
    );

    match action {
        ExpectedAction::SendPacket(packet) => {
            tracing::trace!(?packet, "Handling expected action after connect");
            writer.send(packet).await.expect("Could not send message");
        }
        _ => unreachable!(),
    }

    tracing::trace!("Entering handling loop");
    loop {
        enum GotPacket {
            Incoming(MqttPacket),
            ToSend(SendUsage),
        }

        let action = tokio::select! {
            packet = reader.next() => {
                if let Some(Ok(packet)) = packet {
                    tracing::trace!(?packet, "Received incoming packet");
                    GotPacket::Incoming(packet)
                } else {
                    tracing::trace!("Reader closed, breaking handle loop");
                    break;
                }
            }
            Some(packet) = receiver.recv(), if fsm.is_connected() => {
                tracing::trace!("Received packet to send");
                GotPacket::ToSend(packet)
            }
        };

        tracing::trace!("Processing next action");
        let action = match &action {
            GotPacket::Incoming(packet) => {
                fsm.consume(packet.get_packet().clone()).run(since(start))
            }
            GotPacket::ToSend(send_usage) => match send_usage {
                SendUsage::Publish(packet) => {
                    tracing::trace!("Publishing packet to FSM");
                    let mut publisher =
                        fsm.publish(packet.get_packet().clone().try_into().unwrap());

                    tracing::trace!("Consuming publisher actions");
                    while let Some(action) = publisher.run(since(start)) {
                        tracing::trace!(?action, "Handling action");
                        handle_action(&mut writer, action, &incoming_sender).await;
                    }

                    tracing::trace!("Running FSM");
                    fsm.run(since(start))
                }
                SendUsage::Subscribe(packet) => {
                    tracing::trace!(?packet, "Subscribing in FSM");
                    Some(fsm.subscribe(
                        since(start),
                        packet.get_packet().clone().try_into().unwrap(),
                    ))
                }
            },
        };

        {
            if let Some(action) = action {
                handle_action(&mut writer, action, &incoming_sender).await;
            }
        }
    }

    fsm
}

async fn handle_action<W>(
    writer: &mut FramedWrite<W, MqttPacketCodec>,
    action: ExpectedAction<'_>,
    incoming_sender: &tokio::sync::mpsc::Sender<MqttPacket>,
) where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut writer = std::pin::pin!(writer);

    match action {
        ExpectedAction::SendPacket(mqtt_packet) => {
            writer
                .send(mqtt_packet)
                .await
                .expect("Could not send packet");
        }
        ExpectedAction::ReceivePacket(cloudmqtt_core::client::ReceivePacket::NoFurtherAction(
            received_packet,
        )) => {
            // TODO: Don't await in the FSM loop
            incoming_sender
                .send(MqttPacket::new(received_packet))
                .await
                .unwrap();
        }
        _ => unreachable!(),
    }
}
