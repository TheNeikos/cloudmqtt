//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use core::hash::Hash;
use core::hash::Hasher;
use core::ops::Not;

use mqtt_format::v5::packets::MqttPacket;
use mqtt_format::v5::packets::connack::ConnackReasonCode;
use mqtt_format::v5::packets::connect::MConnect;
use mqtt_format::v5::qos::QualityOfService;
use rustc_hash::FxHasher;
use tracing::trace;

mod packet_identifier_store;
pub use self::packet_identifier_store::PacketIdentifierStore;
pub use self::packet_identifier_store::PacketIdentifierUsage;
pub use self::packet_identifier_store::UsizePacketIdentifierStore;

#[derive(Debug)]
pub struct MqttClientFSM<ClientPacketIdentifierStore = UsizePacketIdentifierStore> {
    data: ClientData,
    connection_state: ConnectionState,
    client_pis: ClientPacketIdentifierStore,
}

impl MqttClientFSM {
    pub fn is_connected(&self) -> bool {
        self.connection_state.is_connected()
    }
}

#[must_use = "Without being run, this will drop the incoming packet"]
pub struct MqttClientConsumer<'c, 'p, CPIS> {
    client: &'c mut MqttClientFSM<CPIS>,
    packet: MqttPacket<'p>,
}

impl<'p, CPIS> MqttClientConsumer<'_, 'p, CPIS>
where
    CPIS: PacketIdentifierStore,
{
    pub fn run(self, current_time: MqttInstant) -> Option<ExpectedAction<'p>> {
        self.client.inner_run(current_time, Some(self.packet), None)
    }
}

enum PublishingState {
    Store,
    Send,
    Done,
}

#[must_use = "Without being run, this will drop the publishing packet"]
pub struct MqttClientPublisher<'c, 'p, CPIS> {
    client: &'c mut MqttClientFSM<CPIS>,
    packet: Option<mqtt_format::v5::packets::publish::MPublish<'p>>,
    state: PublishingState,
}

impl<'p, CPIS> MqttClientPublisher<'_, 'p, CPIS>
where
    CPIS: PacketIdentifierStore,
{
    pub fn run(&mut self, current_time: MqttInstant) -> Option<ExpectedAction<'p>> {
        match &mut self.state {
            PublishingState::Store => {
                if let Some(packet) = &mut self.packet {
                    if packet.quality_of_service != QualityOfService::AtMostOnce {
                        let id = self
                            .client
                            .client_pis
                            .get_next_free(PacketIdentifierUsage::Publish)
                            .unwrap();
                        packet.packet_identifier = Some(id);

                        self.state = PublishingState::Send;
                        return Some(ExpectedAction::StorePacket {
                            id: packet.packet_identifier.unwrap(),
                        });
                    }
                }
                None
            }
            PublishingState::Send => {
                self.state = PublishingState::Done;
                self.client
                    .inner_run(current_time, None, self.packet.take().map(Into::into))
            }
            PublishingState::Done => None,
        }
    }
}

impl Default for MqttClientFSM<UsizePacketIdentifierStore> {
    fn default() -> Self {
        Self::new(UsizePacketIdentifierStore::default())
    }
}

impl<CPIS> MqttClientFSM<CPIS>
where
    CPIS: PacketIdentifierStore,
{
    pub const fn new(client_pis: CPIS) -> MqttClientFSM<CPIS> {
        MqttClientFSM {
            data: ClientData::const_new(0, None, MqttInstant::new(0)),
            connection_state: ConnectionState::Disconnected,
            client_pis,
        }
    }

    pub fn handle_connect<'c>(
        &'_ mut self,
        current_time: MqttInstant,
        connect: MConnect<'c>,
    ) -> ExpectedAction<'c> {
        assert!(matches!(
            self.connection_state,
            ConnectionState::Disconnected
        ));

        if connect.clean_start {
            // TOOD: Handle cleaning of session
        }

        self.data.keep_alive = connect.keep_alive;

        // TODO: Check the Receive Maximum property being sent, and make sure its correct

        // TODO: Check the Maximum Packet Size property

        // TODO: Check Authentication Method is actually as advertised

        let client_id_hash = connect.client_identifier.is_empty().not().then(|| {
            let mut hasher = FxHasher::default();
            connect.client_identifier.hash(&mut hasher);
            hasher.finish()
        });

        if let Some(id) = self.data.client_id_hash {
            assert!(client_id_hash.is_some());
            assert!(client_id_hash.unwrap() == id);
        } else {
            self.data.client_id_hash = client_id_hash;
        }

        self.connection_state = ConnectionState::ConnectingWithoutAuth(ConnectingWithoutAuth {
            connect_sent: current_time,
        });

        ExpectedAction::SendPacket(connect.into())
    }

    pub fn consume<'c, 'p>(
        &'c mut self,
        packet: MqttPacket<'p>,
    ) -> MqttClientConsumer<'c, 'p, CPIS> {
        MqttClientConsumer {
            client: self,
            packet,
        }
    }

    pub fn publish<'c, 'p>(
        &'c mut self,
        packet: mqtt_format::v5::packets::publish::MPublish<'p>,
    ) -> MqttClientPublisher<'c, 'p, CPIS> {
        MqttClientPublisher {
            client: self,
            state: if packet.quality_of_service == QualityOfService::AtMostOnce {
                PublishingState::Send
            } else {
                PublishingState::Store
            },
            packet: Some(packet),
        }
    }

    pub fn subscribe<'p>(
        &mut self,
        current_time: MqttInstant,
        mut packet: mqtt_format::v5::packets::subscribe::MSubscribe<'p>,
    ) -> ExpectedAction<'p> {
        packet.packet_identifier = self
            .client_pis
            .get_next_free(PacketIdentifierUsage::NonPublish)
            .expect("could not get a free packet identifier");

        self.inner_run(current_time, None, Some(packet.into()))
            .expect("inner_run did not return packet as expected")
    }

    pub fn acknowledge<'p>(
        &mut self,
        current_time: MqttInstant,
        AcknowledgeAction(packet_identifier): AcknowledgeAction,
    ) -> ExpectedAction<'p> {
        self.inner_run(
            current_time,
            None,
            Some(
                mqtt_format::v5::packets::puback::MPuback {
                    packet_identifier,
                    reason: mqtt_format::v5::packets::puback::PubackReasonCode::Success,
                    properties: mqtt_format::v5::packets::puback::PubackProperties::new(),
                }
                .into(),
            ),
        )
        .expect("inner_run did not return packet as expected")
    }

    pub fn run(&mut self, current_time: MqttInstant) -> Option<ExpectedAction<'static>> {
        self.inner_run(current_time, None, None)
    }

    #[tracing::instrument(
        skip_all,
        fields(
            current_time = ?current_time,
            to_consume_packet = to_consume_packet.is_some(),
            to_publish_packet = to_send_packet.is_some()
        ),
        ret
    )]
    fn inner_run<'p>(
        &mut self,
        current_time: MqttInstant,
        to_consume_packet: Option<MqttPacket<'p>>,
        to_send_packet: Option<MqttPacket<'p>>,
    ) -> Option<ExpectedAction<'p>> {
        trace!("Doing one state machine step");
        let action = match &self.connection_state {
            ConnectionState::Disconnected => {
                self.data.last_time_run = current_time;
                None
            }
            ConnectionState::ConnectingWithoutAuth { .. } => {
                self.handle_connecting_without_auth(current_time, to_consume_packet)
            }
            ConnectionState::Connected { .. } => {
                self.handle_connected(current_time, to_consume_packet, to_send_packet)
            }
        };

        self.data.last_time_run = current_time;

        action
    }

    fn handle_connected<'p>(
        &mut self,
        current_time: MqttInstant,
        mut to_consume_packet: Option<MqttPacket<'p>>,
        mut to_send_packet: Option<MqttPacket<'p>>,
    ) -> Option<ExpectedAction<'p>> {
        let ConnectionState::Connected(con) = &mut self.connection_state else {
            unreachable!()
        };

        if let Some(incoming_packet) = to_consume_packet.take() {
            match incoming_packet {
                MqttPacket::Publish(
                    publish @ mqtt_format::v5::packets::publish::MPublish {
                        duplicate: _,
                        quality_of_service,
                        retain: _,
                        topic_name: _,
                        packet_identifier,
                        properties: _,
                        payload: _,
                    },
                ) => match quality_of_service {
                    QualityOfService::AtMostOnce => {
                        return Some(ExpectedAction::ReceivePacket(
                            ReceivePacket::NoFurtherAction(MqttPacket::Publish(publish)),
                        ));
                    }
                    QualityOfService::AtLeastOnce => {
                        return Some(ExpectedAction::ReceivePacket(
                            ReceivePacket::AcknowledgeNeeded {
                                // TODO: Dont unwrap()
                                acknowledge: AcknowledgeAction(packet_identifier.unwrap()),
                                packet: MqttPacket::Publish(publish),
                            },
                        ));
                    }
                    QualityOfService::ExactlyOnce => todo!(),
                },

                MqttPacket::Disconnect(_) => todo!(),
                MqttPacket::Puback(puback) => {
                    assert!(self.client_pis.contains(puback.packet_identifier));

                    self.client_pis.release(puback.packet_identifier);

                    return Some(ExpectedAction::ReleasePacket {
                        id: puback.packet_identifier,
                    });
                }
                MqttPacket::Pingresp(mqtt_format::v5::packets::pingresp::MPingresp) => {
                    match &con.ping_state {
                        PingState::WaitingForPingrespSince(_since) => {
                            con.ping_state = PingState::WaitingForElapsed;
                        }
                        PingState::WaitingForElapsed => {
                            panic!("Protocol error, got PingResp without a req");
                        }
                    }
                }
                MqttPacket::Suback(suback) => {
                    assert!(self.client_pis.contains(suback.packet_identifier));

                    self.client_pis.release(suback.packet_identifier);

                    // TODO: Verify that subscriptions don't use QoS higher than we set as maximum
                }
                _ => panic!("Invalid packet received"),
            }
        };

        if let Some(outgoing_publish) = to_send_packet.take() {
            con.last_time_sent = current_time;
            return Some(ExpectedAction::SendPacket(outgoing_publish));
        }

        if self.data.keep_alive > 0 {
            trace!(ping_state = ?con.ping_state, keep_alive = self.data.keep_alive, "Keep alive is non-zero");

            match &con.ping_state {
                PingState::WaitingForElapsed => {
                    trace!(
                        elapsed_since_last_sent = con.last_time_sent.elapsed_seconds(current_time),
                        "Checking if ping is required"
                    );
                    if con.last_time_sent.elapsed_seconds(current_time)
                        >= self.data.keep_alive as u64
                    {
                        trace!("We need to send a ping, doing so now");
                        con.ping_state = PingState::WaitingForPingrespSince(current_time);
                        con.last_time_sent = current_time;

                        return Some(ExpectedAction::SendPacket(MqttPacket::Pingreq(
                            mqtt_format::v5::packets::pingreq::MPingreq,
                        )));
                    }
                }
                PingState::WaitingForPingrespSince(_mqtt_instant) => {}
            }
        }

        None
    }

    fn handle_connecting_without_auth<'p>(
        &mut self,
        _current_time: MqttInstant,
        mut to_consume_packet: Option<MqttPacket<'p>>,
    ) -> Option<ExpectedAction<'p>> {
        let ConnectionState::ConnectingWithoutAuth(conn_without_auth) = &mut self.connection_state
        else {
            unreachable!()
        };
        match to_consume_packet.take() {
            Some(MqttPacket::Connack(connack)) => {
                if connack.reason_code != ConnackReasonCode::Success {
                    panic!("Connection unsuccessful");
                }

                // TODO: Handle session_present flag

                let _server_receive_maximum = connack
                    .properties
                    .receive_maximum()
                    .map(|rm| rm.0.get())
                    .unwrap_or(65_535);

                // TODO: Handle _server_receive_maximum above

                // TODO: Handle maximum QoS

                // TODO: Handle retain available

                // TODO: Handle max packet size

                // TODO: Handle max packet size

                if let Some(keep_alive) = connack.properties.server_keep_alive() {
                    self.data.keep_alive = keep_alive.0;
                }

                let potential_client_id = if let Some(client_identifier) =
                    connack.properties.assigned_client_identifier()
                {
                    let mut hasher = FxHasher::default();
                    client_identifier.0.hash(&mut hasher);
                    self.data.client_id_hash = Some(hasher.finish());
                    Some(client_identifier.0)
                } else {
                    None
                };

                self.connection_state = ConnectionState::Connected(Connected {
                    ping_state: PingState::WaitingForElapsed,
                    last_time_sent: conn_without_auth.connect_sent,
                });

                potential_client_id.map(ExpectedAction::SaveClientIdentifier)
            }
            None => match &self.connection_state {
                ConnectionState::Disconnected => None,
                ConnectionState::ConnectingWithoutAuth { .. } => None,
                ConnectionState::Connected(Connected { .. }) => None,
            },
            p => panic!("Unexpected packet received: {p:?}"),
        }
    }
}

#[derive(Debug)]
pub enum ExpectedAction<'p> {
    SendPacket(MqttPacket<'p>),
    SaveClientIdentifier(&'p str),
    StorePacket {
        id: mqtt_format::v5::variable_header::PacketIdentifier,
    },
    ReleasePacket {
        id: mqtt_format::v5::variable_header::PacketIdentifier,
    },
    ReceivePacket(ReceivePacket<'p>),
}

#[derive(Debug)]
pub enum ReceivePacket<'p> {
    NoFurtherAction(MqttPacket<'p>),
    AcknowledgeNeeded {
        packet: MqttPacket<'p>,
        acknowledge: AcknowledgeAction,
    },
}

#[derive(Debug)]
#[must_use = "AcknowledgeActions need to be sent back to the FSM so that the server considers it received."]
pub struct AcknowledgeAction(mqtt_format::v5::variable_header::PacketIdentifier);

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MqttInstant(u64);

impl MqttInstant {
    pub const fn new(now: u64) -> MqttInstant {
        MqttInstant(now)
    }

    pub fn elapsed_seconds(&self, current_time: MqttInstant) -> u64 {
        self.0.abs_diff(current_time.0)
    }
}

#[derive(Debug)]
pub enum PingState {
    WaitingForElapsed,
    WaitingForPingrespSince(MqttInstant),
}

#[derive(Default, Debug)]
pub struct ClientData {
    keep_alive: u16,
    client_id_hash: Option<u64>,
    last_time_run: MqttInstant,
}

impl ClientData {
    pub const fn const_new(
        keep_alive: u16,
        client_id_hash: Option<u64>,
        last_time_run: MqttInstant,
    ) -> Self {
        Self {
            keep_alive,
            client_id_hash,
            last_time_run,
        }
    }
}

#[derive(Debug)]
struct Connected {
    last_time_sent: MqttInstant,
    ping_state: PingState,
}

#[derive(Debug)]
struct ConnectingWithoutAuth {
    connect_sent: MqttInstant,
}

#[derive(Debug)]
enum ConnectionState {
    Disconnected,
    ConnectingWithoutAuth(ConnectingWithoutAuth),
    Connected(Connected),
}

impl ConnectionState {
    /// Returns `true` if the connection state is [`Connected`].
    ///
    /// [`Connected`]: ConnectionState::Connected
    #[must_use]
    fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(..))
    }
}

#[cfg(test)]
mod tests {
    use super::MqttClientFSM;
    use crate::client::ConnectionState;
    use crate::client::ExpectedAction;

    #[test]
    fn check_simple_connect() {
        let mut fsm = MqttClientFSM::default();

        fsm.handle_connect(
            crate::client::MqttInstant::new(0),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "testing",
                username: None,
                password: None,
                clean_start: false,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 0,
            },
        );

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Connack(
                mqtt_format::v5::packets::connack::MConnack {
                    session_present: false,
                    reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                    properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));
    }

    #[test]
    fn check_ping_request() {
        let mut fsm = MqttClientFSM::default();

        fsm.handle_connect(
            crate::client::MqttInstant::new(0),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "testing",
                username: None,
                password: None,
                clean_start: false,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 10,
            },
        );

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Connack(
                mqtt_format::v5::packets::connack::MConnack {
                    session_present: false,
                    reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                    properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));

        let action = fsm.run(crate::client::MqttInstant::new(5));
        assert!(action.is_none());

        let action = fsm.run(crate::client::MqttInstant::new(9));
        assert!(action.is_none());

        let action = fsm.run(crate::client::MqttInstant::new(10));
        assert!(matches!(
            action,
            Some(ExpectedAction::SendPacket(
                mqtt_format::v5::packets::MqttPacket::Pingreq(..)
            ))
        ));

        let action = fsm.run(crate::client::MqttInstant::new(12));
        assert!(action.is_none());

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Pingresp(
                mqtt_format::v5::packets::pingresp::MPingresp,
            ))
            .run(crate::client::MqttInstant::new(15));
        assert!(action.is_none());

        let action = fsm.run(crate::client::MqttInstant::new(20));
        assert!(matches!(
            action,
            Some(ExpectedAction::SendPacket(
                mqtt_format::v5::packets::MqttPacket::Pingreq(..)
            ))
        ));

        let action = fsm.run(crate::client::MqttInstant::new(22));
        assert!(action.is_none());
    }

    #[test]
    fn check_simple_publish() {
        tracing_subscriber::fmt()
            .with_test_writer()
            .pretty()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        let mut fsm = MqttClientFSM::default();

        fsm.handle_connect(
            crate::client::MqttInstant::new(0),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "testing",
                username: None,
                password: None,
                clean_start: false,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 10,
            },
        );

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Connack(
                mqtt_format::v5::packets::connack::MConnack {
                    session_present: false,
                    reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                    properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));

        let mut publisher = fsm.publish(mqtt_format::v5::packets::publish::MPublish {
            duplicate: false,
            quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
            retain: false,
            topic_name: "foo/bar",
            packet_identifier: None,
            properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
            payload: b"Hello World",
        });

        let action = publisher.run(crate::client::MqttInstant::new(11));
        assert!(
            matches!(
                action,
                Some(ExpectedAction::SendPacket(
                    mqtt_format::v5::packets::MqttPacket::Publish(..)
                ))
            ),
            "Got action: {action:?}"
        );

        let action = publisher.run(crate::client::MqttInstant::new(11));
        assert!(action.is_none(), "Got action: {action:?}");

        let action = fsm.run(crate::client::MqttInstant::new(12));
        assert!(action.is_none(), "Got action: {action:?}");
    }

    #[test]
    fn check_qos1_publish() {
        tracing_subscriber::fmt()
            .with_test_writer()
            .pretty()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        let mut fsm = MqttClientFSM::default();

        fsm.handle_connect(
            crate::client::MqttInstant::new(0),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "testing",
                username: None,
                password: None,
                clean_start: false,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 10,
            },
        );

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Connack(
                mqtt_format::v5::packets::connack::MConnack {
                    session_present: false,
                    reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                    properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));

        let mut publisher = fsm.publish(mqtt_format::v5::packets::publish::MPublish {
            duplicate: false,
            quality_of_service: mqtt_format::v5::qos::QualityOfService::AtLeastOnce,
            retain: false,
            topic_name: "foo/bar",
            packet_identifier: None,
            properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
            payload: b"Hello World",
        });

        let action = publisher.run(crate::client::MqttInstant::new(10));
        assert!(
            matches!(action, Some(ExpectedAction::StorePacket { id }) if id.0.get() == 1),
            "Got action: {action:?}"
        );

        let action = publisher.run(crate::client::MqttInstant::new(10));
        assert!(
            matches!(
                action,
                Some(ExpectedAction::SendPacket(
                    mqtt_format::v5::packets::MqttPacket::Publish(..)
                ))
            ),
            "Got action: {action:?}"
        );

        let action = publisher.run(crate::client::MqttInstant::new(11));
        assert!(action.is_none(), "Got action: {action:?}");

        let action = fsm.run(crate::client::MqttInstant::new(12));
        assert!(action.is_none(), "Got action: {action:?}");

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Puback(
                mqtt_format::v5::packets::puback::MPuback {
                    packet_identifier: mqtt_format::v5::variable_header::PacketIdentifier(
                        1.try_into().unwrap(),
                    ),
                    reason: mqtt_format::v5::packets::puback::PubackReasonCode::Success,
                    properties: mqtt_format::v5::packets::puback::PubackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant(13));
        assert!(
            matches!(
                action,
                Some(ExpectedAction::ReleasePacket { id }) if id.0.get() == 1
            ),
            "Got action: {action:?}"
        );

        let action = fsm.run(crate::client::MqttInstant::new(12));
        assert!(action.is_none(), "Got action: {action:?}");
    }

    #[test]
    fn check_publish_recv() {
        tracing_subscriber::fmt()
            .with_test_writer()
            .pretty()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        let mut fsm = MqttClientFSM::default();

        fsm.handle_connect(
            crate::client::MqttInstant::new(0),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "testing",
                username: None,
                password: None,
                clean_start: false,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 10,
            },
        );

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Connack(
                mqtt_format::v5::packets::connack::MConnack {
                    session_present: false,
                    reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                    properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
                },
            ))
            .run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Publish(
                mqtt_format::v5::packets::publish::MPublish {
                    duplicate: false,
                    quality_of_service: mqtt_format::v5::qos::QualityOfService::AtMostOnce,
                    retain: false,
                    topic_name: "foo",
                    packet_identifier: None,
                    properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
                    payload: &[],
                },
            ))
            .run(crate::client::MqttInstant::new(1));

        // match from hell
        assert!(matches!(
            action,
            Some(ExpectedAction::ReceivePacket(
                crate::client::ReceivePacket::NoFurtherAction { .. }
            ))
        ));

        let action = fsm.run(crate::client::MqttInstant::new(2));
        assert!(action.is_none(), "Got action: {action:?}");

        let action = fsm
            .consume(mqtt_format::v5::packets::MqttPacket::Publish(
                mqtt_format::v5::packets::publish::MPublish {
                    duplicate: false,
                    quality_of_service: mqtt_format::v5::qos::QualityOfService::AtLeastOnce,
                    retain: false,
                    topic_name: "foo",
                    packet_identifier: Some(mqtt_format::v5::variable_header::PacketIdentifier(
                        11.try_into().unwrap(),
                    )),
                    properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
                    payload: &[],
                },
            ))
            .run(crate::client::MqttInstant::new(3));

        // match from hell
        let Some(ExpectedAction::ReceivePacket(crate::client::ReceivePacket::AcknowledgeNeeded {
            packet,
            acknowledge,
        })) = action
        else {
            panic!("Expected ReceivePacket with AcknowledgeNeeded: {action:?}")
        };

        assert!(matches!(
            packet,
            mqtt_format::v5::packets::MqttPacket::Publish { .. }
        ));
        assert!(matches!(acknowledge, crate::client::AcknowledgeAction(pid) if pid.0.get() == 11));

        let action = fsm.acknowledge(crate::client::MqttInstant::new(4), acknowledge);
        assert!(
            matches!(
                action,
                ExpectedAction::SendPacket(mqtt_format::v5::packets::MqttPacket::Puback(..))
            ),
            "Got action: {action:?}"
        );

        let action = fsm.run(crate::client::MqttInstant::new(2));
        assert!(action.is_none(), "Got action: {action:?}");
    }
}
