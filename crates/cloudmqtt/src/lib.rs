//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod codec;
mod topic;

use std::time::Instant;

use cloudmqtt_core::client::ExpectedAction;
use cloudmqtt_core::client::MqttClientFSM;
use cloudmqtt_core::client::MqttInstant;
use codec::BytesMutWriter;
use codec::MqttPacket;
use codec::MqttPacketCodec;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use mqtt_format::v5::packets::subscribe::Subscription;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

fn since(start: Instant) -> MqttInstant {
    MqttInstant::new(start.elapsed().as_secs())
}

enum SendUsage {
    Publish(MqttPacket),
    Subscribe(MqttPacket),
}

pub struct CloudmqttClient {
    client_task: tokio::task::JoinHandle<()>,
    publish_sender: tokio::sync::mpsc::Sender<SendUsage>,
    incoming_messages: Option<tokio::sync::mpsc::Receiver<MqttPacket>>,
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

        let (sender, mut receiver): (tokio::sync::mpsc::Sender<SendUsage>, _) =
            tokio::sync::mpsc::channel(1);

        let (incoming_sender, incoming_receiver): (tokio::sync::mpsc::Sender<MqttPacket>, _) =
            tokio::sync::mpsc::channel(1);

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
                                handle_action(&mut writer, action, &incoming_sender).await;
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
                        handle_action(&mut writer, action, &incoming_sender).await;
                    }
                }
            }
        });

        CloudmqttClient {
            client_task,
            publish_sender: sender,
            incoming_messages: Some(incoming_receiver),
        }
    }

    pub async fn publish(&self, message: impl AsRef<[u8]>, topic: impl AsRef<str>) {
        self.publish_sender
            .send(SendUsage::Publish(MqttPacket::new(
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
            )))
            .await
            .unwrap();
    }

    pub async fn subscribe(&self, topic_filter: impl AsRef<str>) {
        let buf = {
            let sub = Subscription {
                topic_filter: topic_filter.as_ref(),
                options: mqtt_format::v5::packets::subscribe::SubscriptionOptions {
                    quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                    no_local: true,
                    retain_as_published: true,
                    retain_handling: mqtt_format::v5::packets::subscribe::RetainHandling::SendRetainedMessagesAlways,
                }
            };

            let mut bytes = BytesMut::new();
            sub.write(&mut BytesMutWriter(&mut bytes)).unwrap();
            bytes.to_vec()
        };

        self.publish_sender
            .send(SendUsage::Subscribe(MqttPacket::new(
                mqtt_format::v5::packets::MqttPacket::Subscribe(
                    mqtt_format::v5::packets::subscribe::MSubscribe {
                        packet_identifier: mqtt_format::v5::variable_header::PacketIdentifier(
                            1.try_into().unwrap(),
                        ),
                        properties: mqtt_format::v5::packets::subscribe::SubscribeProperties::new(),
                        subscriptions:
                            mqtt_format::v5::packets::subscribe::Subscriptions::parse_complete(&buf)
                                .unwrap(),
                    },
                ),
            )))
            .await
            .expect("Could not subscribe..");
    }

    pub fn receive_messages(&mut self) -> impl Stream<Item = MqttPacket> {
        futures::stream::unfold(self.incoming_messages.take().unwrap(), |mut recv| async {
            recv.recv().await.map(|p| (p, recv))
        })
    }

    pub async fn wait_for_shutdown(&mut self) {
        (&mut self.client_task)
            .await
            .expect("The background task should not panic...")
    }
}

async fn handle_action(
    writer: &mut FramedWrite<&mut tokio::net::tcp::WriteHalf<'_>, MqttPacketCodec>,
    action: ExpectedAction<'_>,
    incoming_sender: &tokio::sync::mpsc::Sender<MqttPacket>,
) {
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
