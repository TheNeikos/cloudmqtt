//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::time::Duration;

use futures::select;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

use super::MqttClient;
use crate::bytes::MqttBytes;
use crate::client::state::OutstandingPackets;
use crate::client::state::TransportWriter;
use crate::client::ConnectState;
use crate::client::SessionState;
use crate::client_identifier::ProposedClientIdentifier;
use crate::codecs::MqttPacketCodecError;
use crate::keep_alive::KeepAlive;
use crate::packets::connack::ConnackPropertiesView;
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

#[must_use]
pub struct Connected {
    pub connack_prop_view: ConnackPropertiesView,
    pub background_task: futures::future::BoxFuture<'static, Result<(), ()>>,
}

impl MqttClient {
    pub async fn connect(
        &self,
        connector: MqttClientConnector,
    ) -> Result<Connected, MqttClientConnectError> {
        type Mcce = MqttClientConnectError;

        let inner_clone = self.inner.clone();
        let mut inner = self.inner.lock().await;
        let (read, write) = tokio::io::split(MqttConnection::from(connector.transport));
        let mut conn_write = FramedWrite::new(write, crate::codecs::MqttPacketCodec);
        let mut conn_read = FramedRead::new(read, crate::codecs::MqttPacketCodec);

        let conn_packet = mqtt_format::v5::packets::connect::MConnect {
            client_identifier: connector.client_identifier.as_str(),
            username: connector.username.as_ref().map(AsRef::as_ref),
            password: connector.password.as_ref().map(AsRef::as_ref),
            clean_start: connector.clean_start.as_bool(),
            will: connector.will.as_ref().map(|w| w.as_ref()),
            properties: connector.properties.as_ref(),
            keep_alive: connector.keep_alive.as_u16(),
        };

        conn_write
            .send(mqtt_format::v5::packets::MqttPacket::Connect(conn_packet))
            .await
            .map_err(Mcce::Send)?;

        let Some(maybe_connack) = conn_read.next().await else {
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

            let (sender, heartbeat_receiver) = futures::channel::mpsc::channel(1);
            let conn_write = TransportWriter::new(conn_write, sender);

            let (conn_read_sender, conn_read_recv) = futures::channel::oneshot::channel();

            let connect_client_state = ConnectState {
                session_present: connack.session_present,
                receive_maximum: connack.properties.receive_maximum().map(|rm| rm.0),
                maximum_qos: connack.properties.maximum_qos().map(|mq| mq.0),
                retain_available: connack.properties.retain_available().map(|ra| ra.0),
                maximum_packet_size: connack.properties.maximum_packet_size().map(|mps| mps.0),
                topic_alias_maximum: connack.properties.topic_alias_maximum().map(|tam| tam.0),
                keep_alive: connack
                    .properties
                    .server_keep_alive()
                    .map(|ska| {
                        std::num::NonZeroU16::try_from(ska.0)
                            .map(KeepAlive::Seconds)
                            .unwrap_or(KeepAlive::Disabled)
                    })
                    .unwrap_or(connector.keep_alive),
                conn_write,
                conn_read_recv,
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

            let keep_alive = connect_client_state.keep_alive;

            inner.connection_state = Some(connect_client_state);
            inner.session_state = Some(SessionState {
                client_identifier,
                outstanding_packets: OutstandingPackets::empty(),
            });

            let connack_prop_view =
                crate::packets::connack::ConnackPropertiesView::try_from(maybe_connack)
                    .expect("An already matched value suddenly changed?");

            let background_task = async move {
                let receiving_inner = inner_clone.clone();
                let receiving = crate::client::receive::handle_background_receiving(
                    receiving_inner,
                    conn_read,
                    conn_read_sender,
                );

                let heartbeat_inner = inner_clone;

                let heartbeat = if let KeepAlive::Seconds(time) = keep_alive {
                    handle_heartbeats(
                        heartbeat_receiver,
                        Duration::from_secs(time.get().into()),
                        heartbeat_inner,
                    )
                    .left_future()
                } else {
                    tracing::info!(
                        "Keep Alive is disabled, will not send PingReq packets automatically"
                    );
                    futures::future::ok(()).right_future()
                };

                tokio::try_join!(receiving, heartbeat)
                    .map(drop)
                    .map_err(drop)
            }
            .boxed();

            return Ok(Connected {
                connack_prop_view,
                background_task,
            });
        }

        // TODO: Do something with error code

        todo!()
    }
}

async fn handle_heartbeats(
    mut heartbeat_receiver: futures::channel::mpsc::Receiver<()>,
    duration: Duration,
    heartbeat_inner: std::sync::Arc<futures::lock::Mutex<super::InnerClient>>,
) -> Result<(), ()> {
    let mut timeout = futures_timer::Delay::new(duration).fuse();
    loop {
        select! {
            heartbeat = heartbeat_receiver.next() => match heartbeat {
                None => break,
                Some(_) => {
                    timeout = futures_timer::Delay::new(duration).fuse();
                },
            },
            _ = timeout => {
                let mut inner = heartbeat_inner.lock().await;
                let inner = &mut *inner;
                let Some(conn_state) = inner.connection_state.as_mut() else {
                    todo!();
                };

                // We make sure that this won't deadlock in the send method
                conn_state.conn_write.send(
                    mqtt_format::v5::packets::MqttPacket::Pingreq(mqtt_format::v5::packets::pingreq::MPingreq)
                ).await.unwrap();
            }
        }
    }
    Ok(())
}
