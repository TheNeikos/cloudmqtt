//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod client;
mod codec;
mod topic;

use codec::BytesMutWriter;
use codec::MqttPacket;
use futures::Stream;
use mqtt_format::v5::packets::subscribe::Subscription;
use tokio_util::bytes::BytesMut;

enum SendUsage {
    Publish(MqttPacket),
    Subscribe(MqttPacket),
}

pub struct CloudmqttClient {
    core_client: crate::client::CoreClient,
    incoming_messages: Option<tokio::sync::mpsc::Receiver<MqttPacket>>,
}

impl CloudmqttClient {
    pub async fn new(address: String) -> CloudmqttClient {
        let socket = tokio::net::lookup_host(address)
            .await
            .expect("Could not lookup DNS")
            .next()
            .expect("DNS resolved to no addresses");

        let connection = tokio::net::TcpStream::connect(socket)
            .await
            .expect("Could not connect");

        let (incoming_sender, incoming_receiver): (tokio::sync::mpsc::Sender<MqttPacket>, _) =
            tokio::sync::mpsc::channel(1);

        let core_client = crate::client::CoreClient::new(connection, incoming_sender.clone());

        CloudmqttClient {
            core_client,
            incoming_messages: Some(incoming_receiver),
        }
    }

    pub async fn publish(&self, message: impl AsRef<[u8]>, topic: impl AsRef<str>) {
        self.core_client
            .publish(MqttPacket::new(
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

        self.core_client
            .subscribe(MqttPacket::new(
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
            ))
            .await
    }

    pub fn receive_messages(&mut self) -> impl Stream<Item = MqttPacket> {
        futures::stream::unfold(self.incoming_messages.take().unwrap(), |mut recv| async {
            recv.recv().await.map(|p| (p, recv))
        })
    }

    pub async fn wait_for_shutdown(&mut self) {
        std::future::pending::<()>().await;
    }
}
