//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v5::integers::VARIABLE_INTEGER_MAX;

#[derive(Debug)]
pub struct MqttPayload(Vec<u8>);

impl AsRef<[u8]> for MqttPayload {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttPayloadError {
    #[error(
        "MQTT Publish payloads must always be less than {} bytes. Given: {given}",
        VARIABLE_INTEGER_MAX
    )]
    Length { given: usize },
}

impl TryFrom<Vec<u8>> for MqttPayload {
    type Error = MqttPayloadError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() > VARIABLE_INTEGER_MAX as usize {
            Err(MqttPayloadError::Length { given: value.len() })
        } else {
            Ok(MqttPayload(value))
        }
    }
}
