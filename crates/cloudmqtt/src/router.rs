//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use dashmap::DashMap;

use crate::SubscriptionId;
use crate::SubscriptionSink;
use crate::topic::TopicFilterBuf;
use crate::topic::TopicNameBuf;

pub struct Router {
    _join_handle: tokio::task::JoinHandle<()>,
    next_subscription_id: std::sync::atomic::AtomicU64,
    subscriptions: Arc<DashMap<SubscriptionId, SubscriptionSink>>,
    subscription_topics: Arc<DashMap<TopicFilterBuf, Vec<SubscriptionId>>>,
}

impl Router {
    pub fn new(
        mut incoming_receiver: tokio::sync::mpsc::Receiver<crate::codec::MqttPacket>,
    ) -> Self {
        let subscriptions =
            std::sync::Arc::new(dashmap::DashMap::<SubscriptionId, SubscriptionSink>::new());
        let subscription_topics =
            std::sync::Arc::new(dashmap::DashMap::<TopicFilterBuf, Vec<SubscriptionId>>::new());

        let join_handle = tokio::task::spawn({
            let subscriptions = subscriptions.clone();
            let subscription_topics = subscription_topics.clone();
            async move {
                while let Some(next_packet) = incoming_receiver.recv().await {
                    tracing::info!("Received packet");

                    let mqtt_format::v5::packets::MqttPacket::Publish(
                        mqtt_format::v5::packets::publish::MPublish { topic_name, .. },
                    ) = next_packet.get_packet()
                    else {
                        panic!("Received non-publish packet in router");
                    };

                    let topic_name_buf = match TopicNameBuf::new(topic_name) {
                        Ok(buf) => buf,
                        Err(error) => {
                            tracing::warn!(?error, "Invalid topic name");
                            continue;
                        }
                    };

                    let Some(subscription_ids) = subscription_topics
                        .iter()
                        .find(|r| topic_name_buf.matches(r.key()))
                    else {
                        tracing::debug!(topic = ?topic_name_buf, "Did not find any subscription id for topic");
                        continue;
                    };

                    for subscription_id in subscription_ids.value() {
                        let Some(sender) = subscriptions
                            .get(subscription_id)
                            .map(|r| r.value().clone())
                        else {
                            tracing::debug!(topic = ?topic_name_buf, "Did not find any subscription for topic");
                            continue;
                        };

                        if let Err(error) = sender.send(next_packet.clone()).await {
                            tracing::error!(?error, "TODO");
                        }
                    }
                }
            }
        });

        Self {
            _join_handle: join_handle,
            next_subscription_id: std::sync::atomic::AtomicU64::new(0),
            subscriptions,
            subscription_topics,
        }
    }

    pub(crate) fn add_subscription_sink(&self, sink: SubscriptionSink) -> SubscriptionId {
        let subscription_id = SubscriptionId(
            self.next_subscription_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        self.subscriptions.insert(subscription_id, sink);

        subscription_id
    }

    pub(crate) fn add_subscription_to_topic(
        &self,
        subscription_id: SubscriptionId,
        topic_filter: &str,
    ) {
        self.subscription_topics
            .entry(TopicFilterBuf::new(topic_filter).unwrap())
            .or_default()
            .push(subscription_id);
    }
}
