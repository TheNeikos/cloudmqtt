//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::num::NonZeroU16;

use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;
use mqtt_format::v5::integers::VARIABLE_INTEGER_MAX;
use mqtt_format::v5::packets::publish::MPublish;
use tokio_util::codec::Framed;

use crate::bytes::MqttBytes;
use crate::client_identifier::ProposedClientIdentifier;
use crate::codecs::MqttPacketCodec;
use crate::codecs::MqttPacketCodecError;
use crate::keep_alive::KeepAlive;
use crate::packets::connack::ConnackPropertiesView;
use crate::payload::MqttPayload;
use crate::qos::QualityOfService;
use crate::string::MqttString;
use crate::transport::MqttConnectTransport;
use crate::transport::MqttConnection;

#[derive(Debug, PartialEq, Eq)]
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
    client_identifier: ProposedClientIdentifier,
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
        client_identifier: ProposedClientIdentifier,
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

    pub fn properties_mut(&mut self) -> &mut crate::packets::connect::ConnectProperties {
        &mut self.properties
    }
}

struct ConnectState {
    session_present: bool,
    receive_maximum: Option<NonZeroU16>,
    maximum_qos: Option<mqtt_format::v5::qos::MaximumQualityOfService>,
    retain_available: Option<bool>,
    topic_alias_maximum: Option<u16>,
    maximum_packet_size: Option<u32>,
    conn: Framed<MqttConnection, MqttPacketCodec>,

    next_packet_identifier: std::num::NonZeroU16,
}

struct SessionState {
    client_identifier: MqttString,
    outstanding_packets: OutstandingPackets,
}

struct OutstandingPackets {
    packet_ident_order: Vec<std::num::NonZeroU16>,
    outstanding_packets:
        std::collections::BTreeMap<std::num::NonZeroU16, crate::packets::MqttPacket>,
}

impl OutstandingPackets {
    pub fn empty() -> Self {
        Self {
            packet_ident_order: Vec::new(),
            outstanding_packets: std::collections::BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, ident: std::num::NonZeroU16, packet: crate::packets::MqttPacket) {
        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );

        self.packet_ident_order.push(ident);
        let removed = self.outstanding_packets.insert(ident, packet);

        debug_assert!(removed.is_none());
    }

    pub fn update_by_id(&mut self, ident: std::num::NonZeroU16, packet: crate::packets::MqttPacket) {
        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );

        let removed = self.outstanding_packets.insert(ident, packet);

        debug_assert!(removed.is_some());
    }

    pub fn exists_outstanding_packet(&self, ident: std::num::NonZeroU16) -> bool {
        self.outstanding_packets.contains_key(&ident)
    }

    pub fn iter_in_send_order(
        &self,
    ) -> impl Iterator<Item = (std::num::NonZeroU16, &crate::packets::MqttPacket)> {
        self.packet_ident_order
            .iter()
            .flat_map(|id| self.outstanding_packets.get(id).map(|p| (*id, p)))
    }

    pub fn remove_by_id(&mut self, id: std::num::NonZeroU16) {
        // Vec::retain() preserves order
        self.packet_ident_order.retain(|&elm| elm != id);
        self.outstanding_packets.remove(&id);

        debug_assert_eq!(
            self.packet_ident_order.len(),
            self.outstanding_packets.len()
        );
    }
}

struct InnerClient {
    connection_state: Option<ConnectState>,
    session_state: Option<SessionState>,
}

pub struct MqttClient {
    inner: Mutex<InnerClient>,
}

impl MqttClient {
    #[allow(clippy::new_without_default)]
    pub fn new() -> MqttClient {
        MqttClient {
            inner: Mutex::new(InnerClient {
                connection_state: None,
                session_state: None,
            }),
        }
    }

    pub async fn connect(
        &self,
        connector: MqttClientConnector,
    ) -> Result<ConnackPropertiesView, MqttClientConnectError> {
        type Mcce = MqttClientConnectError;

        let mut inner = self.inner.lock().await;
        let mut conn = tokio_util::codec::Framed::new(
            MqttConnection::from(connector.transport),
            MqttPacketCodec,
        );

        let conn_packet = mqtt_format::v5::packets::connect::MConnect {
            client_identifier: connector.client_identifier.as_str(),
            username: connector.username.as_ref().map(AsRef::as_ref),
            password: connector.password.as_ref().map(AsRef::as_ref),
            clean_start: connector.clean_start.as_bool(),
            will: connector.will.as_ref().map(|w| w.as_ref()),
            properties: connector.properties.as_ref(),
            keep_alive: connector.keep_alive.as_u16(),
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
            let can_use_auth = connector.properties.authentication_data.is_some();
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

            if connack.session_present && connector.clean_start == CleanStart::Yes {
                return Err(MqttClientConnectError::ServerProtocolError {
                    reason: "MQTT-3.2.2-2",
                });
            }

            let connect_client_state = ConnectState {
                session_present: connack.session_present,
                receive_maximum: connack.properties.receive_maximum().map(|rm| rm.0),
                maximum_qos: connack.properties.maximum_qos().map(|mq| mq.0),
                retain_available: connack.properties.retain_available().map(|ra| ra.0),
                maximum_packet_size: connack.properties.maximum_packet_size().map(|mps| mps.0),
                topic_alias_maximum: connack.properties.topic_alias_maximum().map(|tam| tam.0),
                conn,
                next_packet_identifier: std::num::NonZeroU16::MIN,
            };

            let assigned_client_identifier = connack.properties.assigned_client_identifier();

            let client_identifier: MqttString;

            if let Some(aci) = assigned_client_identifier {
                if connector.client_identifier
                    == ProposedClientIdentifier::PotentiallyServerProvided
                {
                    client_identifier = MqttString::try_from(aci.0).map_err(|_mse| {
                        MqttClientConnectError::ServerProtocolError {
                            reason: "MQTT-1.5.4",
                        }
                    })?;
                } else {
                    return Err(MqttClientConnectError::ServerProtocolError {
                        reason: "MQTT-3.2.2.3.7",
                    });
                }
            } else {
                client_identifier = match connector.client_identifier {
                    ProposedClientIdentifier::PotentiallyServerProvided => {
                        return Err(MqttClientConnectError::ServerProtocolError {
                            reason: "MQTT-3.2.2.3.7",
                        });
                    }
                    ProposedClientIdentifier::MinimalRequired(mr) => mr.into_inner(),
                    ProposedClientIdentifier::PotentiallyAccepted(pa) => pa.into_inner(),
                };
            }

            inner.connection_state = Some(connect_client_state);
            inner.session_state = Some(SessionState {
                client_identifier,
                outstanding_packets: OutstandingPackets::empty(),
            });

            return Ok(
                crate::packets::connack::ConnackPropertiesView::try_from(maybe_connack)
                    .expect("An already matched value suddenly changed?"),
            );
        }

        // TODO: Do something with error code

        todo!()
    }

    #[tracing::instrument(skip(self, payload), fields(payload_length = payload.as_ref().len()))]
    pub async fn publish(
        &self,
        topic: crate::topic::MqttTopic,
        _qos: QualityOfService,
        retain: bool,
        payload: MqttPayload,
    ) -> Result<(), ()> {
        let mut inner = self.inner.lock().await;

        let Some(conn_state) = &mut inner.connection_state else {
            return Err(());
        };

        if conn_state.retain_available.unwrap_or(true) && retain {
            return Err(());
        }

        let publish = MPublish {
            duplicate: false,
            quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
            retain,
            topic_name: topic.as_ref(),
            packet_identifier: None,
            properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
            payload: payload.as_ref(),
        };

        let packet = mqtt_format::v5::packets::MqttPacket::Publish(publish);

        let maximum_packet_size = conn_state
            .maximum_packet_size
            .unwrap_or(VARIABLE_INTEGER_MAX);

        if packet.binary_size() > maximum_packet_size {
            return Err(());
        }

        tracing::trace!(%maximum_packet_size, "Packet size");

        conn_state.conn.send(packet).await.unwrap();

        tracing::trace!("Finished publishing");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::MqttClient;

    static_assertions::assert_impl_all!(MqttClient: Send, Sync);
}
