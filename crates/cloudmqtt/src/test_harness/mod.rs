//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::collections::HashMap;

use error::TestHarnessError;

mod broker;
mod client;
pub mod error;

#[derive(Debug)]
pub struct TestHarness {
    brokers: HashMap<String, broker::Broker>,
    clients: HashMap<String, client::Client>,
    runtime: tokio::runtime::Runtime,
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHarness {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        Self {
            brokers: HashMap::default(),
            clients: HashMap::default(),
            runtime,
        }
    }

    pub fn sleep(&self, duration: std::time::Duration) -> Result<(), TestHarnessError> {
        let _rt = self.runtime.enter();
        let fut = tokio::time::sleep(duration);
        self.runtime.block_on(fut);
        Ok(())
    }

    pub fn start_broker(&mut self, name: String) -> Result<(), error::TestHarnessError> {
        let broker = broker::Broker::new(name.clone());
        self.brokers.insert(name, broker);
        Ok(())
    }

    pub fn create_client(&mut self, name: String) -> Result<(), error::TestHarnessError> {
        let _rt = self.runtime.enter();
        let client = client::Client::new(name.clone());
        self.clients.insert(name, client);
        Ok(())
    }

    pub fn connect_client_to_broker(
        &mut self,
        client_name: String,
        broker_name: String,
    ) -> Result<(), error::TestHarnessError> {
        let broker = self
            .brokers
            .get_mut(&broker_name)
            .ok_or(error::TestHarnessError::BrokerNotFound(broker_name))?;
        let client = self
            .clients
            .get_mut(&client_name)
            .ok_or(error::TestHarnessError::ClientNotFound(client_name))?;

        self.runtime.block_on(client.connect_to(broker))
    }

    pub fn publish(
        &mut self,
        client_name: String,
        payload: String,
        topic: String,
    ) -> Result<(), error::TestHarnessError> {
        let client = self
            .clients
            .get_mut(&client_name)
            .ok_or(error::TestHarnessError::ClientNotFound(client_name))?;

        self.runtime.block_on(client.publish(payload, topic))
    }

    pub fn publish_to_client(
        &mut self,
        broker_name: String,
        client_name: String,
        payload: String,
        topic: String,
    ) -> Result<(), error::TestHarnessError> {
        let broker = self
            .brokers
            .get_mut(&broker_name)
            .ok_or(error::TestHarnessError::BrokerNotFound(broker_name))?;

        self.runtime.block_on(broker.send(
            &client_name,
            mqtt_format::v5::packets::MqttPacket::Publish(
                mqtt_format::v5::packets::publish::MPublish {
                    duplicate: false,
                    quality_of_service: mqtt_format::v5::qos::QualityOfService::AtLeastOnce,
                    retain: false,
                    topic_name: &topic,
                    packet_identifier: None,
                    properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
                    payload: payload.as_bytes(),
                },
            ),
        ))
    }

    pub fn wait_for_publish_on_broker(
        &self,
        broker_name: String,
        client_name: String,
        expected_payload: String,
        expected_topic: String,
    ) -> Result<(), error::TestHarnessError> {
        let broker = self
            .brokers
            .get(&broker_name)
            .ok_or(error::TestHarnessError::BrokerNotFound(broker_name))?;

        let fut = broker.has_received_packet(&client_name, |packet: &crate::codec::MqttPacket| {
            match packet.get_packet() {
                mqtt_format::v5::packets::MqttPacket::Publish(
                    mqtt_format::v5::packets::publish::MPublish {
                        topic_name,
                        payload,
                        ..
                    },
                ) => {
                    if *topic_name != expected_topic {
                        tracing::warn!("Unexpected topic: {topic_name} != {expected_topic}");
                        return false;
                    }

                    let Ok(payload) = std::str::from_utf8(payload) else {
                        tracing::warn!("Payload not valid UTF8");
                        return false;
                    };

                    if payload != expected_payload {
                        tracing::warn!("Unexpected payload: {payload} != {expected_payload}");
                        return false;
                    }

                    true
                }
                _ => false,
            }
        });

        match self.runtime.block_on(fut) {
            Ok(true) => Ok(()),
            Ok(false) => {
                todo!()
            }
            Err(error) => Err(error),
        }
    }

    pub fn wait_for_connect_on_broker(
        &self,
        broker_name: String,
        client_name: String,
        client_identifier: String,
    ) -> Result<(), TestHarnessError> {
        let broker = self
            .brokers
            .get(&broker_name)
            .ok_or(error::TestHarnessError::BrokerNotFound(broker_name))?;

        let fut = broker.has_received_packet(&client_name, |p: &crate::codec::MqttPacket| match p
            .get_packet()
        {
            mqtt_format::v5::packets::MqttPacket::Connect(connect) => {
                let b = connect.client_identifier == client_identifier;

                if !b {
                    tracing::error!(
                        "Client identifier wrong: {} != {}",
                        connect.client_identifier,
                        client_identifier
                    );
                }

                b
            }
            _ => false,
        });

        let received_connect_packet = self.runtime.block_on(fut)?;
        let _ = broker;

        if !received_connect_packet {
            return Err(TestHarnessError::PacketNotReceived("connect".to_string()));
        }

        Ok(())
    }
}
