//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
use std::sync::Arc;

use mqtt_format::v3::qos::MQualityOfService;
use mqtt_format::v3::will::MLastWill;

use super::ClientId;

#[derive(Debug, Clone)]
pub struct MqttMessage {
    author_id: Arc<ClientId>,
    payload: Vec<u8>,
    topic: String,
    retain: bool,
    qos: MQualityOfService,
}

impl MqttMessage {
    pub fn new(
        author_id: Arc<ClientId>,
        payload: Vec<u8>,
        topic: String,
        retain: bool,
        qos: MQualityOfService,
    ) -> Self {
        Self {
            author_id,
            payload,
            topic,
            retain,
            qos,
        }
    }

    pub fn from_last_will(will: &MLastWill, author_id: Arc<ClientId>) -> MqttMessage {
        MqttMessage {
            topic: will.topic.to_string(),
            payload: will.payload.to_vec(),
            qos: will.qos,
            retain: will.retain,
            author_id,
        }
    }

    pub fn qos(&self) -> MQualityOfService {
        self.qos
    }

    pub fn retain(&self) -> bool {
        self.retain
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }

    pub fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    pub fn author_id(&self) -> &ClientId {
        self.author_id.as_ref()
    }

    pub fn set_qos(&mut self, qos: MQualityOfService) {
        self.qos = qos;
    }
}
