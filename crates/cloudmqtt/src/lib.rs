//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod codec;

use std::time::Instant;

use cloudmqtt_core::client::ExpectedAction;
use cloudmqtt_core::client::MqttClientFSM;
use cloudmqtt_core::client::MqttInstant;
use codec::MqttPacket;
use codec::MqttPacketCodec;
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

fn since(start: Instant) -> MqttInstant {
    MqttInstant::new(start.elapsed().as_secs())
}

pub struct CloudmqttClient {
    client_task: tokio::task::JoinHandle<()>,
    publish_sender: tokio::sync::mpsc::Sender<MqttPacket>,
}

impl CloudmqttClient {
    pub async fn new(address: String) -> CloudmqttClient {
        let socket = tokio::net::lookup_host(address)
            .await
            .expect("Could not lookup DNS")
            .next()
            .expect("DNS resolved to no addresses");

        let mut connection = tokio::net::TcpStream::connect(socket)
            .await
            .expect("Could not connect");

        let mut fsm = MqttClientFSM::default();

        let start = Instant::now();

        let (sender, mut receiver): (
            tokio::sync::mpsc::Sender<MqttPacket>,
            tokio::sync::mpsc::Receiver<MqttPacket>,
        ) = tokio::sync::mpsc::channel(1);

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
                    ToPublish(MqttPacket),
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
                        GotPacket::ToPublish(packet)
                    }
                };

                let action = match &action {
                    GotPacket::Incoming(packet) => {
                        fsm.consume(packet.get_packet().clone()).run(since(start))
                    }
                    GotPacket::ToPublish(packet) => fsm
                        .publish(packet.get_packet().clone().try_into().unwrap())
                        .run(since(start)),
                };

                {
                    if let Some(action) = action {
                        match action {
                            ExpectedAction::SendPacket(mqtt_packet) => {
                                writer
                                    .send(mqtt_packet)
                                    .await
                                    .expect("Could not send packet");
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        });

        CloudmqttClient {
            client_task,
            publish_sender: sender,
        }
    }

    pub async fn publish(&self, message: impl AsRef<[u8]>, topic: impl AsRef<str>) {
        self.publish_sender
            .send(MqttPacket::new(
                mqtt_format::v5::packets::MqttPacket::Publish(
                    mqtt_format::v5::packets::publish::MPublish {
                        duplicate: false,
                        quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                        retain: false,
                        topic_name: topic.as_ref(),
                        packet_identifier: None,
                        properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
                        payload: message.as_ref(),
                    },
                ),
            ))
            .await
            .unwrap();
    }

    pub async fn wait_for_shutdown(&mut self) {
        (&mut self.client_task)
            .await
            .expect("The background task should not panic...")
    }
}
