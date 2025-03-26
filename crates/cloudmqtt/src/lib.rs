//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod client;
mod codec;
mod error;
mod router;
pub mod topic;

use codec::BytesMutWriter;
use codec::MqttPacket;
use error::Error;
use futures::Stream;
use tokio_util::bytes::BytesMut;

enum SendUsage {
    Publish(MqttPacket),
    Subscribe(MqttPacket),
}

pub struct CloudmqttClient {
    core_client: crate::client::CoreClient,
    router: crate::router::Router,
}

impl CloudmqttClient {
    pub async fn new(address: String) -> Result<CloudmqttClient, Error> {
        let socket = tokio::net::lookup_host(address)
            .await
            .map_err(Error::DnsLookup)?
            .next()
            .ok_or(Error::DnsNoAddrs)?;

        let connection = tokio::net::TcpStream::connect(socket)
            .await
            .map_err(Error::TcpConnect)?;

        let (incoming_sender, incoming_receiver): (tokio::sync::mpsc::Sender<MqttPacket>, _) =
            tokio::sync::mpsc::channel(1);

        let core_client = crate::client::CoreClient::new(connection, incoming_sender.clone());

        let router = crate::router::Router::new(incoming_receiver);

        Ok(CloudmqttClient {
            core_client,
            router,
        })
    }

    pub async fn publish(
        &self,
        message: impl AsRef<[u8]>,
        topic: impl AsRef<str>,
    ) -> Result<(), Error> {
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

    pub async fn subscribe(&self, topic_filter: impl AsRef<str>) -> Result<Subscription, Error> {
        self.subscription_builder()
            .with_subscription(topic_filter)
            .build()
            .await
    }

    pub fn subscription_builder(&self) -> SubscriptionBuilder<'_> {
        SubscriptionBuilder {
            client: self,
            topic_filters: Vec::new(),
        }
    }

    pub async fn wait_for_shutdown(&mut self) {
        std::future::pending::<()>().await;
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct SubscriptionId(u64);

type SubscriptionSink = tokio::sync::mpsc::Sender<MqttPacket>;

pub struct Subscription {
    _subscription_id: SubscriptionId,
    receiver: tokio::sync::mpsc::Receiver<MqttPacket>,
}

impl Stream for Subscription {
    type Item = MqttPacket;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

pub struct SubscriptionBuilder<'a> {
    client: &'a CloudmqttClient,
    topic_filters: Vec<String>,
}

impl SubscriptionBuilder<'_> {
    pub fn with_subscription(mut self, topic_filter: impl AsRef<str>) -> Self {
        self.topic_filters.push(topic_filter.as_ref().to_string());
        self
    }

    pub async fn build(self) -> Result<Subscription, Error> {
        let buf = {
            let mut bytes = BytesMut::new();

            for topic_filter in self.topic_filters.iter() {
                let sub = mqtt_format::v5::packets::subscribe::Subscription {
                    topic_filter,
                    options: mqtt_format::v5::packets::subscribe::SubscriptionOptions {
                        quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                        no_local: true,
                        retain_as_published: true,
                        retain_handling: mqtt_format::v5::packets::subscribe::RetainHandling::SendRetainedMessagesAlways,
                    }
                };

                sub.write(&mut BytesMutWriter(&mut bytes))
                    .map_err(Error::WriteBuffer)?;
            }

            bytes.to_vec()
        };

        self.client
            .core_client
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
            .await;

        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let subscription_id = self.client.router.add_subscription_sink(sender);

        for topic_filter in self.topic_filters.iter() {
            self.client
                .router
                .add_subscription_to_topic(subscription_id, topic_filter.as_ref());
        }

        Ok(Subscription {
            _subscription_id: subscription_id,
            receiver,
        })
    }
}
