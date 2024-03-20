//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use arc_swap::ArcSwap;
use futures::{stream::FuturesUnordered, StreamExt};
use mqtt_format::v3::{
    qos::MQualityOfService, subscription_acks::MSubscriptionAck,
    subscription_request::MSubscriptionRequests,
};
use tracing::{debug, trace};

use crate::server::{ClientId, MqttMessage};

use super::handler::{AllowAllSubscriptions, SubscriptionHandler};

// foo/barr/# => vec![Named, Named, MultiWildcard]
// /foo/barr/# => vec![Empty, ... ]
// /devices/+/temperature

#[derive(Debug, Clone)]
struct TopicName(VecDeque<String>);

impl TopicName {
    fn parse_from(topic: &str) -> TopicName {
        TopicName(topic.split('/').map(|t| t.to_owned()).collect())
    }

    fn get_matches<'a>(
        &'a self,
        idx: usize,
        routing: &'a SubscriptionTopic,
    ) -> Box<dyn Iterator<Item = &'a ClientSubscription> + 'a> {
        let multi_wild = routing
            .children
            .get(&TopicFilter::MultiWildcard)
            .into_iter()
            .flat_map(|child| child.subscriptions.iter())
            .inspect(|sub| trace!(?sub, "Matching MultiWildcard topic"));

        let single_wild = routing
            .children
            .get(&TopicFilter::SingleWildcard)
            .into_iter()
            .flat_map(move |child| self.get_matches(idx + 1, child))
            .inspect(|sub| trace!(?sub, "Matching SingleWildcard topic"));

        let nested_named = self
            .0
            .get(idx)
            .and_then(|topic_level| {
                routing
                    .children
                    .get(&TopicFilter::Named(topic_level.to_string()))
            })
            .map(move |child| self.get_matches(idx + 1, child));

        let current_named = if idx == self.0.len() {
            Some(routing.subscriptions.iter())
        } else {
            None
        };

        Box::new(
            multi_wild
                .chain(single_wild)
                .chain(nested_named.into_iter().flatten())
                .chain(current_named.into_iter().flatten()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TopicFilter {
    MultiWildcard,
    SingleWildcard,
    Named(String),
}

impl TopicFilter {
    pub fn parse_from(topic: String) -> VecDeque<TopicFilter> {
        topic
            .split('/')
            .map(|piece| match piece {
                "#" => TopicFilter::MultiWildcard,
                "+" => TopicFilter::SingleWildcard,
                name => TopicFilter::Named(name.to_owned()),
            })
            .collect()
    }
}

pub(crate) struct SubscriptionManager<SubH> {
    subscription_handler: SubH,
    subscriptions: Arc<ArcSwap<SubscriptionTopic>>,
}

impl SubscriptionManager<AllowAllSubscriptions> {
    pub(crate) fn new() -> Self {
        SubscriptionManager {
            subscription_handler: AllowAllSubscriptions,
            subscriptions: Arc::default(),
        }
    }
}

impl<SH: SubscriptionHandler> SubscriptionManager<SH> {
    pub(crate) fn with_subscription_handler<NSH: SubscriptionHandler>(
        self,
        new_subscription_handler: NSH,
    ) -> SubscriptionManager<NSH> {
        SubscriptionManager {
            subscriptions: self.subscriptions,
            subscription_handler: new_subscription_handler,
        }
    }
    pub(crate) async fn subscribe(
        &self,
        client: Arc<ClientInformation>,
        subscriptions: MSubscriptionRequests<'_>,
    ) -> Vec<MSubscriptionAck> {
        debug!(?client, ?subscriptions, "Subscribing client");
        let sub_changes: Vec<_> = subscriptions
            .into_iter()
            .map(|sub| {
                let client = client.clone();
                async move {
                    let topic_levels: VecDeque<TopicFilter> =
                        TopicFilter::parse_from(sub.topic.to_string());

                    let sub_resp = self
                        .subscription_handler
                        .allow_subscription(client.client_id.clone(), sub)
                        .await;

                    let ack = match sub_resp {
                        None => MSubscriptionAck::Failure,
                        Some(MQualityOfService::AtMostOnce) => {
                            MSubscriptionAck::MaximumQualityAtMostOnce
                        }
                        Some(MQualityOfService::AtLeastOnce) => {
                            MSubscriptionAck::MaximumQualityAtLeastOnce
                        }
                        Some(MQualityOfService::ExactlyOnce) => {
                            MSubscriptionAck::MaximumQualityExactlyOnce
                        }
                    };

                    let client_sub = sub_resp.map(|qos| ClientSubscription { qos, client });

                    (topic_levels, client_sub, ack)
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect()
            .await;

        self.subscriptions.rcu(|old_table| {
            let mut subs = SubscriptionTopic::clone(old_table);

            for (topic, client, _) in sub_changes.clone() {
                if let Some(client) = client {
                    subs.add_subscription(topic, client);
                }
            }

            subs
        });

        sub_changes.into_iter().map(|(_, _, ack)| ack).collect()
    }

    pub(crate) async fn route_message(&self, message: MqttMessage) {
        debug!(?message, "Routing message");
        let routing = self.subscriptions.load();

        let topic = message.topic();

        let topic_names = TopicName::parse_from(topic);

        let matches = topic_names.get_matches(0, &routing).collect::<Vec<_>>();

        debug!(?matches, "Sending to matching subscriptions");

        for sub in matches {
            let mut message = message.clone();
            message.set_qos(message.qos().min(sub.qos));
            sub.publish_message(message);
        }
    }
}

pub(crate) struct ClientInformation {
    pub(crate) client_id: Arc<ClientId>,
    pub(crate) client_sender: tokio::sync::mpsc::UnboundedSender<MqttMessage>,
}

impl std::fmt::Debug for ClientInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientInformation")
            .field("client_id", &self.client_id)
            .field("client_sender", &"..")
            .finish()
    }
}

#[derive(Debug, Clone)]
struct ClientSubscription {
    client: Arc<ClientInformation>,
    qos: MQualityOfService,
}

impl PartialEq for ClientSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.client.client_id == other.client.client_id
    }
}

impl ClientSubscription {
    fn publish_message(&self, packet: MqttMessage) {
        let _ = self.client.client_sender.send(packet);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct SubscriptionTopic {
    subscriptions: Vec<ClientSubscription>,
    children: HashMap<TopicFilter, SubscriptionTopic>,
}

impl SubscriptionTopic {
    fn add_subscription(&mut self, mut topic: VecDeque<TopicFilter>, client: ClientSubscription) {
        match topic.pop_front() {
            None => self.subscriptions.push(client),
            Some(filter) => {
                self.children
                    .entry(filter)
                    .or_default()
                    .add_subscription(topic, client);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mqtt_format::v3::qos::MQualityOfService;

    use crate::server::{subscriptions::TopicFilter, ClientId};

    use super::{ClientInformation, ClientSubscription, SubscriptionTopic};

    macro_rules! build_subs {
        (@topic "#") => {
            TopicFilter::MultiWildcard
        };
        (@topic "+") => {
            TopicFilter::SingleWildcard
        };
        (@topic $topic:literal) => {
            TopicFilter::Named(String::from($topic))
        };

        (@leaf subscriptions: [$($clients:expr),* $(,)?], children: { $($topic:tt => { $($rest:tt)*})* }  ) => {
            SubscriptionTopic {
                subscriptions: vec![$($clients),*],
                children: [$(
                    (build_subs!(@topic $topic) , build_subs!(@leaf $($rest)*) ),
                )*].into_iter().collect(),
            }
        };
        ( $($topic:tt => { $($rest:tt)*})+ ) => {
            SubscriptionTopic {
                subscriptions: vec![],
                children: [$(
                    (build_subs!(@topic $topic) , build_subs!(@leaf $($rest)*) ),
                )+].into_iter().collect(),
            }
        };
    }

    fn client_subscription(qos: MQualityOfService) -> ClientSubscription {
        let (client_sender, _) = tokio::sync::mpsc::unbounded_channel();
        ClientSubscription {
            client: Arc::new(ClientInformation {
                client_id: Arc::new(ClientId::new(String::from("test-sub"))),
                client_sender,
            }),
            qos,
        }
    }

    #[test]
    fn check_macro_builder() {
        let real = SubscriptionTopic {
            subscriptions: vec![],
            children: [(
                TopicFilter::SingleWildcard,
                SubscriptionTopic {
                    subscriptions: vec![client_subscription(MQualityOfService::AtLeastOnce)],
                    children: Default::default(),
                },
            )]
            .into_iter()
            .collect(),
        };

        let built = build_subs! {
            "+" => {
                subscriptions: [ client_subscription(MQualityOfService::AtLeastOnce) ],
                children: {}
            }
        };

        assert_eq!(built, real);
    }

    #[test]
    fn check_simple_apply_change() {
        let check = SubscriptionTopic {
            subscriptions: vec![],
            children: [(
                TopicFilter::Named(String::from("foo")),
                SubscriptionTopic {
                    subscriptions: vec![client_subscription(MQualityOfService::AtLeastOnce)],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        };

        let new = {
            let mut new = SubscriptionTopic::default();
            new.add_subscription(
                vec![TopicFilter::Named(String::from("foo"))].into(),
                client_subscription(MQualityOfService::AtLeastOnce),
            );
            new
        };

        assert_eq!(check, new);
    }

    #[test]
    fn check_full_merging() {
        let check = build_subs! {
            "foo" => {
                subscriptions: [
                    client_subscription(MQualityOfService::AtLeastOnce),
                    client_subscription(MQualityOfService::AtLeastOnce),
                ],
                children: {
                    "+" => {
                        subscriptions: [ client_subscription(MQualityOfService::AtMostOnce) ],
                        children: {}
                    }
                }
            }
        };

        let new = {
            let mut new = build_subs! {
                "foo" => {
                    subscriptions: [
                        client_subscription(MQualityOfService::AtLeastOnce)
                    ],
                    children: {}
                }
            };
            new.add_subscription(
                vec![TopicFilter::Named("foo".to_owned())].into(),
                client_subscription(MQualityOfService::AtLeastOnce),
            );
            new.add_subscription(
                TopicFilter::parse_from("foo/+".to_string()),
                client_subscription(MQualityOfService::AtMostOnce),
            );
            new
        };

        assert_eq!(check, new);
    }
}
