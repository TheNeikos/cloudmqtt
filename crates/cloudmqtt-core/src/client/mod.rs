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
    current_state: ClientState,
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
            current_state: ClientState::Disconnected,
            to_consume_packet: None,
        }
    }

    pub fn handle_connect<'c>(&'_ mut self, connect: MConnect<'c>) -> ExpectedAction<'c> {
        assert!(matches!(self.current_state, ClientState::Disconnected));

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

        self.current_state = ClientState::ConnectingWithoutAuth;

        ExpectedAction::SendPacket(connect.into())
    }

    pub fn consume(&mut self, message: MqttPacket<'p>) {
        assert!(self.to_consume_packet.is_none());
        self.to_consume_packet = Some(message);
    }

    pub fn run(&mut self) -> Option<ExpectedAction<'p>> {
        match &self.current_state {
            ClientState::Disconnected => {
                panic!("Should prepare connecting before consuming messages")
            }
            ClientState::ConnectingWithoutAuth => match self.to_consume_packet.take() {
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

                    self.current_state = ClientState::Connected;
                    potential_client_id.map(ExpectedAction::SaveClientIdentifier)
                }
                None => match &self.current_state {
                    ClientState::Disconnected => None,
                    ClientState::ConnectingWithoutAuth => None,
                    ClientState::Connected => None,
                },
                p => panic!("Unexpected packet received: {p:?}"),
            },
            ClientState::Connected => None,
        }
    }
}

#[expect(clippy::large_enum_variant)]
pub enum ExpectedAction<'p> {
    SendPacket(MqttPacket<'p>),
    SaveClientIdentifier(&'p str),
}

#[derive(Default)]
pub struct ClientData {
    keep_alive: u16,
    client_id_hash: Option<u64>,
}

enum ClientState {
    Disconnected,
    ConnectingWithoutAuth,
    Connected,
}
