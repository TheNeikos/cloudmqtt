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
use mqtt_format::v3::{qos::MQualityOfService, subscription_request::MSubscriptionRequests};
use tracing::{debug, trace};

use crate::server::{ClientId, MqttMessage};

// foo/barr/# => vec![Named, Named, MultiWildcard]
// /foo/barr/# => vec![Empty, ... ]
// /devices/+/temperature

#[derive(Debug, Clone)]
struct TopicName(VecDeque<String>);

impl TopicName {
    fn parse_from(topic: &str) -> TopicName {
        TopicName(topic.split('/').map(|t| t.to_owned()).collect())
    }

    fn pop_front(&mut self) -> Option<String> {
        self.0.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn get_matches<'a>(
        &'a self,
        idx: usize,
        routing: &'a SubscriptionTopic,
    ) -> Box<dyn Iterator<Item = &'a ClientSubscription> + 'a> {
        let mwild = routing
            .children
            .get(&TopicFilter::MultiWildcard)
            .into_iter()
            .flat_map(|child| child.subscriptions.iter())
            .inspect(|sub| trace!(?sub, "Matching MultiWildcard topic"));

        let swild = routing
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
            mwild
                .chain(swild)
                .chain(nested_named.into_iter().flatten())
                .chain(current_named.into_iter().flatten()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TopicFilter {
    MultiWildcard,
    SingleWildcard,
    Named(String),
}

impl TopicFilter {
    fn parse_from(topic: String) -> VecDeque<TopicFilter> {
        topic
            .split('/')
            .map(|piece| match piece {
                "#" => TopicFilter::MultiWildcard,
                "+" => TopicFilter::SingleWildcard,
                name => TopicFilter::Named(name.to_owned()),
            })
            .collect()
    }

    pub fn try_into_named(self) -> Result<String, Self> {
        if let Self::Named(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionManager {
    subscriptions: Arc<ArcSwap<SubscriptionTopic>>,
}

impl SubscriptionManager {
    pub fn new() -> SubscriptionManager {
        SubscriptionManager {
            subscriptions: Default::default(),
        }
    }

    pub async fn subscribe(
        &self,
        client: Arc<ClientInformation>,
        subscriptions: MSubscriptionRequests<'_>,
    ) {
        debug!(?client, ?subscriptions, "Subscribing client");
        let sub_changes: Vec<_> = subscriptions
            .into_iter()
            .map(|sub| {
                let topic_levels: VecDeque<TopicFilter> =
                    TopicFilter::parse_from(sub.topic.to_string());
                let client_sub = ClientSubscription {
                    qos: sub.qos,
                    client: client.clone(),
                };

                (topic_levels, client_sub)
            })
            .collect();

        self.subscriptions.rcu(|old_table| {
            let mut subs = SubscriptionTopic::clone(old_table);

            for (topic, client) in sub_changes.clone() {
                subs.add_subscription(topic, client);
            }

            subs
        });
    }

    pub async fn route_message(&self, message: MqttMessage) {
        debug!(?message, "Routing message");
        let routing = self.subscriptions.load();

        let qos = message.qos();
        let topic = message.topic();

        let topic_names = TopicName::parse_from(topic);

        let matches = topic_names
            .get_matches(0, &routing)
            .into_iter()
            .collect::<Vec<_>>();

        debug!(?matches, "Sending to matching subscriptions");

        for sub in matches {
            sub.publish_message(message.clone());
        }
    }
}

#[derive(Debug)]
pub struct ClientInformation {
    pub client_id: Arc<ClientId>,
    pub client_sender: tokio::sync::mpsc::UnboundedSender<MqttMessage>,
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
        self.client.client_sender.send(packet);
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
    use crate::topics::{ClientSubscription, TopicFilter};

    use super::SubscriptionTopic;

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

    #[test]
    fn check_macro_builder() {
        let real = SubscriptionTopic {
            subscriptions: vec![],
            children: [(
                TopicFilter::SingleWildcard,
                SubscriptionTopic {
                    subscriptions: vec![ClientSubscription {
                        qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                    }],
                    children: Default::default(),
                },
            )]
            .into_iter()
            .collect(),
        };

        let built = build_subs! {
            "+" => {
                subscriptions: [ ClientSubscription { qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce, } ],
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
                    subscriptions: vec![ClientSubscription {
                        qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                    }],
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
                ClientSubscription {
                    qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                },
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
                    ClientSubscription {
                        qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                    },
                    ClientSubscription {
                        qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                    },
                ],
                children: {
                    "+" => {
                        subscriptions: [
                            ClientSubscription {
                                qos: mqtt_format::v3::qos::MQualityOfService::AtMostOnce,
                            },
                        ],
                        children: {}
                    }
                }
            }
        };

        let new = {
            let mut new = build_subs! {
                "foo" => {
                    subscriptions: [
                        ClientSubscription {
                            qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                        },
                    ],
                    children: {}
                }
            };
            new.add_subscription(
                vec![TopicFilter::Named("foo".to_owned())].into(),
                ClientSubscription {
                    qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                },
            );
            new.add_subscription(
                TopicFilter::parse_from("foo/+".to_string()),
                ClientSubscription {
                    qos: mqtt_format::v3::qos::MQualityOfService::AtMostOnce,
                },
            );
            new
        };

        assert_eq!(check, new);
    }
}
