//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::str::FromStr;

use crate::string::MqttString;
use crate::string::MqttStringError;

#[derive(Debug)]
pub struct MqttTopic(MqttString);

impl AsRef<str> for MqttTopic {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttTopicError {
    #[error(transparent)]
    String(#[from] MqttStringError),

    #[error("MQTT Topics are not allowed to be empty")]
    Empty,

    #[error("MQTT Topics are not allowed to contain a NULL (U+0000) character")]
    Null,

    #[error("MQTT Topics are not allowed to contain MQTT wildcard characters ('#' or '+')")]
    Wildcard,
}

impl FromStr for MqttTopic {
    type Err = MqttTopicError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(MqttTopicError::Empty);
        }

        if s.contains('\0') {
            return Err(MqttTopicError::Null);
        }

        if s.contains(['#', '+']) {
            return Err(MqttTopicError::Wildcard);
        }

        // MQTTString checks the length for us
        Ok(MqttTopic(MqttString::from_str(s)?))
    }
}

impl TryFrom<String> for MqttTopic {
    type Error = MqttTopicError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&str> for MqttTopic {
    type Error = MqttTopicError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}
