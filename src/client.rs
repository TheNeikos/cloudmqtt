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

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::connect::ConnectProperties,
    anker: "_Toc3901046",
    pub struct ConnectProperties {
        (anker: "_Toc3901048")
        session_expiry_interval: SessionExpiryInterval with setter = u32,

        (anker: "_Toc3901049")
        receive_maximum: ReceiveMaximum with setter = u32,

        (anker: "_Toc3901050")
        maximum_packet_size: MaximumPacketSize with setter = u32,

        (anker: "_Toc3901051")
        topic_alias_maximum: TopicAliasMaximum with setter = u32,

        (anker: "_Toc3901052")
        request_response_information: RequestResponseInformation with setter = u8,

        (anker: "_Toc3901053")
        request_problem_information: RequestProblemInformation with setter = u8,

        (anker: "_Toc3901054")
        user_properties: UserProperties<'a> with setter = crate::properties::UserProperty,

        (anker: "_Toc3901055")
        authentication_method: AuthenticationMethod<'a> with setter = String,

        (anker: "_Toc3901056")
        authentication_data: AuthenticationData<'a> with setter = Vec<u8>,
    }
}

pub struct MqttClientConnector {
    transport: MqttConnectTransport,
    client_identifier: ClientIdentifier,
    clean_start: CleanStart,
    keep_alive: KeepAlive,
    properties: ConnectProperties,
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
            properties: ConnectProperties::new(),
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

    pub fn properties_mut(&mut self) -> &mut ConnectProperties {
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
