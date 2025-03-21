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
}

impl Router {
    pub fn new(
        mut incoming_receiver: tokio::sync::mpsc::Receiver<crate::codec::MqttPacket>,
        subscriptions: Arc<DashMap<SubscriptionId, SubscriptionSink>>,
        subscription_topics: Arc<DashMap<TopicFilterBuf, Vec<SubscriptionId>>>,
    ) -> Self {
        let join_handle = tokio::task::spawn(async move {
            while let Some(next_packet) = incoming_receiver.recv().await {
                tracing::info!("Received packet");

                let mqtt_format::v5::packets::MqttPacket::Publish(
                    mqtt_format::v5::packets::publish::MPublish { topic_name, .. },
                ) = next_packet.get_packet()
                else {
                    panic!("Received non-publish packet in router");
                };

                let topic_name_buf = TopicNameBuf::new(topic_name).unwrap(); // TODO

                let Some(subscription_ids) = subscription_topics
                    .iter()
                    .find(|r| topic_name_buf.matches(r.key()))
                else {
                    todo!()
                };

                for subscription_id in subscription_ids.value() {
                    let Some(sender) = subscriptions.get(subscription_id) else {
                        todo!()
                    };

                    if let Err(error) = sender.value().send(next_packet.clone()).await {
                        tracing::error!(?error, "TODO");
                    }
                }
            }
        });

        Self {
            _join_handle: join_handle,
        }
    }
}
