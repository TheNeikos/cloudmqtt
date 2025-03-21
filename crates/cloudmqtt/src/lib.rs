//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod client;
mod codec;
mod router;
pub mod topic;

use std::sync::Arc;

use codec::BytesMutWriter;
use codec::MqttPacket;
use dashmap::DashMap;
use futures::Stream;
use tokio_util::bytes::BytesMut;
use topic::TopicFilterBuf;

enum SendUsage {
    Publish(MqttPacket),
    Subscribe(MqttPacket),
}

pub struct CloudmqttClient {
    core_client: crate::client::CoreClient,
    _router: crate::router::Router,

    next_subscription_id: std::sync::atomic::AtomicU64,
    subscriptions: Arc<DashMap<SubscriptionId, SubscriptionSink>>,
    subscription_topics: Arc<DashMap<TopicFilterBuf, Vec<SubscriptionId>>>,
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

        let subscriptions = std::sync::Arc::new(dashmap::DashMap::new());
        let subscription_topics = std::sync::Arc::new(dashmap::DashMap::new());
        let router = crate::router::Router::new(
            incoming_receiver,
            subscriptions.clone(),
            subscription_topics.clone(),
        );

        CloudmqttClient {
            core_client,
            next_subscription_id: std::sync::atomic::AtomicU64::new(0),
            subscriptions,
            subscription_topics,
            _router: router,
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

    pub async fn subscribe(&self, topic_filter: impl AsRef<str>) -> Subscription {
        let buf = {
            let sub = mqtt_format::v5::packets::subscribe::Subscription {
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
            .await;

        let subscription_id = SubscriptionId(
            self.next_subscription_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.subscriptions.insert(subscription_id, sender);
        self.subscription_topics
            .entry(TopicFilterBuf::new(topic_filter.as_ref()).unwrap())
            .or_default()
            .push(subscription_id);
        Subscription {
            _subscription_id: subscription_id,
            receiver,
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
