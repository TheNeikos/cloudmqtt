//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::Framed;

use crate::bytes::MqttBytes;
use crate::client_identifier::ClientIdentifier;
use crate::codecs::MqttPacketCodec;
use crate::codecs::MqttPacketCodecError;
use crate::keep_alive::KeepAlive;
use crate::string::MqttString;
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

#[derive(typed_builder::TypedBuilder)]
pub struct MqttWill {
    #[builder(default = crate::packets::connect::ConnectWillProperties::new())]
    properties: crate::packets::connect::ConnectWillProperties,
    topic: MqttString,
    payload: MqttBytes,
    qos: mqtt_format::v5::qos::QualityOfService,
    retain: bool,
}

impl MqttWill {
    pub fn get_properties_mut(&mut self) -> &mut crate::packets::connect::ConnectWillProperties {
        &mut self.properties
    }
}

impl MqttWill {
    fn as_ref(&self) -> mqtt_format::v5::packets::connect::Will<'_> {
        mqtt_format::v5::packets::connect::Will {
            properties: self.properties.as_ref(),
            topic: self.topic.as_ref(),
            payload: self.payload.as_ref(),
            will_qos: self.qos,
            will_retain: self.retain,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttClientConnectError {
    #[error("An error occured while encoding or sending an MQTT Packet")]
    Send(#[source] MqttPacketCodecError),

    #[error("An error occured while decoding or receiving an MQTT Packet")]
    Receive(#[source] MqttPacketCodecError),

    #[error("The transport unexpectedly closed")]
    TransportUnexpectedlyClosed,

    #[error("The server sent a response with a protocol error: {reason}")]
    ServerProtocolError { reason: &'static str },
}

pub struct MqttClientConnector {
    transport: MqttConnectTransport,
    client_identifier: ClientIdentifier,
    clean_start: CleanStart,
    keep_alive: KeepAlive,
    properties: crate::packets::connect::ConnectProperties,
    username: Option<MqttString>,
    password: Option<MqttBytes>,
    will: Option<MqttWill>,
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
            username: None,
            password: None,
            will: None,
        }
    }

    pub fn with_username(&mut self, username: MqttString) -> &mut Self {
        self.username = Some(username);
        self
    }

    pub fn with_password(&mut self, password: MqttBytes) -> &mut Self {
        self.password = Some(password);
        self
    }

    pub fn with_will(&mut self, will: MqttWill) -> &mut Self {
        self.will = Some(will);
        self
    }

    pub async fn connect(self) -> Result<MqttClient, MqttClientConnectError> {
        type Mcce = MqttClientConnectError;
        let mut conn =
            tokio_util::codec::Framed::new(MqttConnection::from(self.transport), MqttPacketCodec);

        let conn_packet = mqtt_format::v5::packets::connect::MConnect {
            client_identifier: self.client_identifier.as_str(),
            username: self.username.as_ref().map(AsRef::as_ref),
            password: self.password.as_ref().map(AsRef::as_ref),
            clean_start: self.clean_start.as_bool(),
            will: self.will.as_ref().map(|w| w.as_ref()),
            properties: self.properties.as_ref(),
            keep_alive: self.keep_alive.as_u16(),
        };

        conn.send(mqtt_format::v5::packets::MqttPacket::Connect(conn_packet))
            .await
            .map_err(Mcce::Send)?;

        let Some(maybe_connack) = conn.next().await else {
            return Err(Mcce::TransportUnexpectedlyClosed);
        };

        let maybe_connack = match maybe_connack {
            Ok(maybe_connack) => maybe_connack,
            Err(e) => {
                return Err(Mcce::Receive(e));
            }
        };

        let connack = loop {
            let can_use_auth = self.properties.authentication_data.is_some();
            let _auth = match maybe_connack.get() {
                mqtt_format::v5::packets::MqttPacket::Connack(connack) => break connack,
                mqtt_format::v5::packets::MqttPacket::Auth(auth) => {
                    if can_use_auth {
                        auth
                    } else {
                        // MQTT-4.12.0-6
                        return Err(Mcce::ServerProtocolError {
                            reason: "MQTT-4.12.0-6",
                        });
                    }
                }
                _ => {
                    return Err(MqttClientConnectError::ServerProtocolError {
                        reason: "MQTT-3.1.4-5",
                    });
                }
            };

            // TODO: Use user-provided method to authenticate further

            todo!()
        };

        // TODO: Timeout here if the server doesn't respond

        if connack.reason_code == mqtt_format::v5::packets::connack::ConnackReasonCode::Success {
            // TODO: Read properties, configure client

            return Ok(MqttClient { _conn: conn });
        }

        // TODO: Do something with error code

        todo!()
    }

    pub fn properties_mut(&mut self) -> &mut crate::packets::connect::ConnectProperties {
        &mut self.properties
    }
}

pub struct MqttClient {
    _conn: Framed<MqttConnection, MqttPacketCodec>,
}

impl MqttClient {}
