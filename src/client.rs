//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use crate::client_identifier::ClientIdentifier;
use crate::keep_alive::KeepAlive;
use crate::transport::MqttConnectTransport;
use crate::transport::MqttConnection;

pub enum CleanStart {
    No,
    Yes,
}

impl CleanStart {
    pub fn as_bool(&self) -> bool {
        match self {
            CleanStart::No => false,
            CleanStart::Yes => true,
        }
    }
}

pub struct MqttClientConnector {
    transport: MqttConnectTransport,
    client_identifier: ClientIdentifier,
    clean_start: CleanStart,
    keep_alive: KeepAlive,
    properties: crate::packets::connect::ConnectProperties,
}

impl MqttClientConnector {
    pub fn new(
        transport: MqttConnectTransport,
        client_identifier: ClientIdentifier,
        clean_start: CleanStart,
        keep_alive: KeepAlive,
    ) -> MqttClientConnector {
        MqttClientConnector {
            transport,
            client_identifier,
            clean_start,
            keep_alive,
            properties: crate::packets::connect::ConnectProperties::new(),
        }
    }

    pub async fn connect(self) -> Result<MqttClient, ()> {
        let conn: MqttConnection = self.transport.into();

        let conn_packet = mqtt_format::v5::packets::connect::MConnect {
            client_identifier: self.client_identifier.as_str(),
            username: None,
            password: None,
            clean_start: self.clean_start.as_bool(),
            will: None,
            properties: self.properties.as_ref(),
            keep_alive: self.keep_alive.as_u16(),
        };

        todo!()
    }

    pub fn properties_mut(&mut self) -> &mut crate::packets::connect::ConnectProperties {
        &mut self.properties
    }
}

pub struct MqttClient {
    conn: MqttConnection,
}

impl MqttClient {
    pub(crate) fn new_with_connection(conn: MqttConnection) -> Self {
        MqttClient { conn }
    }
}
