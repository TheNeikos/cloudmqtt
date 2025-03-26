//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::time::Instant;

use cloudmqtt_core::client::ExpectedAction;
use cloudmqtt_core::client::MqttClientFSM;
use cloudmqtt_core::client::MqttInstant;
use futures::SinkExt;
use futures::StreamExt;
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
    #[allow(dead_code)]
    client_task: tokio::task::JoinHandle<()>,
    publish_sender: tokio::sync::mpsc::Sender<SendUsage>,
}

impl CoreClient {
    pub fn new(
        mut connection: tokio::net::TcpStream,
        incoming_sender: tokio::sync::mpsc::Sender<MqttPacket>,
    ) -> Self {
        let (sender, mut receiver): (tokio::sync::mpsc::Sender<SendUsage>, _) =
            tokio::sync::mpsc::channel(1);

        let mut fsm = MqttClientFSM::default();

        let start = Instant::now();

        let client_task = tokio::task::spawn(async move {
            let (reader, mut writer) = connection.split();

            let mut writer = FramedWrite::new(&mut writer, MqttPacketCodec);
            let mut reader = FramedRead::new(reader, MqttPacketCodec);

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
                ExpectedAction::SendPacket(mqtt_packet) => {
                    writer
                        .send(mqtt_packet)
                        .await
                        .expect("Could not send message");
                }
                _ => unreachable!(),
            }
            loop {
                enum GotPacket {
                    Incoming(MqttPacket),
                    ToSend(SendUsage),
                }

                let action = tokio::select! {
                    packet = reader.next() => {
                        if let Some(Ok(packet)) = packet {
                            GotPacket::Incoming(packet)
                        } else {
                            println!("We're out, bye!");
                            break;
                        }
                    }
                    Some(packet) = receiver.recv(), if fsm.is_connected() => {
                        GotPacket::ToSend(packet)
                    }
                };

                let action = match &action {
                    GotPacket::Incoming(packet) => {
                        fsm.consume(packet.get_packet().clone()).run(since(start))
                    }
                    GotPacket::ToSend(send_usage) => match send_usage {
                        SendUsage::Publish(packet) => {
                            let mut publisher =
                                fsm.publish(packet.get_packet().clone().try_into().unwrap());

                            while let Some(action) = publisher.run(since(start)) {
                                if let Err(error) =
                                    handle_action(&mut writer, action, &incoming_sender).await
                                {
                                    tracing::warn!(?error, "Could not handle action");
                                }
                            }

                            fsm.run(since(start))
                        }
                        SendUsage::Subscribe(packet) => Some(fsm.subscribe(
                            since(start),
                            packet.get_packet().clone().try_into().unwrap(),
                        )),
                    },
                };

                {
                    if let Some(action) = action {
                        if let Err(error) =
                            handle_action(&mut writer, action, &incoming_sender).await
                        {
                            tracing::warn!(?error, "Could not handle action");
                        }
                    }
                }
            }
        });

        Self {
            client_task,
            publish_sender: sender,
        }
    }

    pub async fn publish(&self, packet: MqttPacket) -> Result<(), Error> {
        self.publish_sender
            .send(SendUsage::Publish(packet))
            .await
            .map_err(|_| Error::InternalChannelClosed)
    }

    pub async fn subscribe(&self, packet: MqttPacket) {
        self.publish_sender
            .send(SendUsage::Subscribe(packet))
            .await
            .expect("Could not subscribe..");
    }
}

async fn handle_action(
    writer: &mut FramedWrite<&mut tokio::net::tcp::WriteHalf<'_>, MqttPacketCodec>,
    action: ExpectedAction<'_>,
    incoming_sender: &tokio::sync::mpsc::Sender<MqttPacket>,
) -> Result<(), Error> {
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
                .send(MqttPacket::new(received_packet)?)
                .await
                .unwrap();
        }
        _ => unreachable!(),
    }
    Ok(())
}
