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
use rustc_hash::FxHasher;

pub struct MqttClientFSM<'p> {
    data: ClientData,
    connection_state: ConnectionState,
    to_consume_packet: Option<MqttPacket<'p>>,
}

impl Default for MqttClientFSM<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'p> MqttClientFSM<'p> {
    pub fn new() -> MqttClientFSM<'p> {
        MqttClientFSM {
            data: ClientData::default(),
            connection_state: ConnectionState::Disconnected,
            to_consume_packet: None,
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

    pub fn consume(&mut self, message: MqttPacket<'p>) {
        assert!(self.to_consume_packet.is_none());
        self.to_consume_packet = Some(message);
    }

    pub fn run(&mut self, current_time: MqttInstant) -> Option<ExpectedAction<'p>> {
        let action = match &self.connection_state {
            ConnectionState::Disconnected => {
                self.data.last_time_run = current_time;
                None
            }
            ConnectionState::ConnectingWithoutAuth { .. } => {
                self.handle_connecting_without_auth(current_time)
            }
            ConnectionState::Connected { .. } => self.handle_connected(current_time),
        };

        self.data.last_time_run = current_time;

        action
    }

    fn handle_connected(&mut self, current_time: MqttInstant) -> Option<ExpectedAction<'p>> {
        let ConnectionState::Connected(con) = &mut self.connection_state else {
            unreachable!()
        };

        if self.data.keep_alive > 0 {
            match &con.ping_state {
                PingState::WaitingForElapsed => {
                    if con.last_time_sent.elapsed_seconds(current_time)
                        >= self.data.keep_alive as u64
                    {
                        con.ping_state = PingState::WaitingForPingrespSince(current_time);

                        return Some(ExpectedAction::SendPacket(MqttPacket::Pingreq(
                            mqtt_format::v5::packets::pingreq::MPingreq,
                        )));
                    }
                }
                PingState::WaitingForPingrespSince(_mqtt_instant) => {}
            }
        }

        let incoming_packet = self.to_consume_packet.take()?;

        match incoming_packet {
            MqttPacket::Publish(_) => todo!(),
            MqttPacket::Disconnect(_) => todo!(),
            MqttPacket::Pingresp(mqtt_format::v5::packets::pingresp::MPingresp) => {
                match &con.ping_state {
                    PingState::WaitingForPingrespSince(_since) => {
                        con.ping_state = PingState::WaitingForElapsed;
                        None
                    }
                    PingState::WaitingForElapsed => {
                        panic!("Protocol error, got PingResp without a req");
                    }
                }
            }
            _ => panic!("Invalid packet received"),
        }
    }

    fn handle_connecting_without_auth(
        &mut self,
        _current_time: MqttInstant,
    ) -> Option<ExpectedAction<'p>> {
        let ConnectionState::ConnectingWithoutAuth(conn_without_auth) = &mut self.connection_state
        else {
            unreachable!()
        };
        match self.to_consume_packet.take() {
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

#[expect(clippy::large_enum_variant)]
pub enum ExpectedAction<'p> {
    SendPacket(MqttPacket<'p>),
    SaveClientIdentifier(&'p str),
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MqttInstant(u64);

impl MqttInstant {
    pub fn new(now: u64) -> MqttInstant {
        MqttInstant(now)
    }

    fn elapsed_seconds(&self, current_time: MqttInstant) -> u64 {
        self.0.abs_diff(current_time.0)
    }
}

pub enum PingState {
    WaitingForElapsed,
    WaitingForPingrespSince(MqttInstant),
}

#[derive(Default)]
pub struct ClientData {
    keep_alive: u16,
    client_id_hash: Option<u64>,
    last_time_run: MqttInstant,
}

struct Connected {
    last_time_sent: MqttInstant,
    ping_state: PingState,
}

struct ConnectingWithoutAuth {
    connect_sent: MqttInstant,
}

enum ConnectionState {
    Disconnected,
    ConnectingWithoutAuth(ConnectingWithoutAuth),
    Connected(Connected),
}

#[cfg(test)]
mod tests {
    use super::MqttClientFSM;
    use crate::client::ConnectionState;
    use crate::client::ExpectedAction;

    #[test]
    fn check_simple_connect() {
        let mut fsm = MqttClientFSM::new();

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

        fsm.consume(mqtt_format::v5::packets::MqttPacket::Connack(
            mqtt_format::v5::packets::connack::MConnack {
                session_present: false,
                reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
            },
        ));

        let action = fsm.run(crate::client::MqttInstant::new(0));
        assert!(action.is_none());

        assert!(matches!(
            fsm.connection_state,
            ConnectionState::Connected { .. }
        ));
    }

    #[test]
    fn check_ping_request() {
        let mut fsm = MqttClientFSM::new();

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

        fsm.consume(mqtt_format::v5::packets::MqttPacket::Connack(
            mqtt_format::v5::packets::connack::MConnack {
                session_present: false,
                reason_code: mqtt_format::v5::packets::connack::ConnackReasonCode::Success,
                properties: mqtt_format::v5::packets::connack::ConnackProperties::new(),
            },
        ));

        let action = fsm.run(crate::client::MqttInstant::new(0));
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

        fsm.consume(mqtt_format::v5::packets::MqttPacket::Pingresp(
            mqtt_format::v5::packets::pingresp::MPingresp,
        ));

        let action = fsm.run(crate::client::MqttInstant::new(15));
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
}
