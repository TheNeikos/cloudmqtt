//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, thiserror::Error)]
#[allow(clippy::large_enum_variant)]
pub enum TestHarnessError {
    #[error("Broker '{}' not found", .0)]
    BrokerNotFound(String),

    #[error("Client '{}' not found", .0)]
    ClientNotFound(String),

    #[error("Internal channel error")]
    Channel,

    #[error("Client errored")]
    Client(#[source] crate::error::Error),

    #[error("Codec error")]
    Codec(#[source] crate::codec::MqttPacketCodecError),

    #[error("Stream for '{}' closed", .0)]
    StreamClosed(String),

    #[error("Received not expected packet: '{:?}'", .got)]
    PacketNotExpected { got: Box<crate::codec::MqttPacket> },

    #[error("Unexpected client identifier: got {}, expected {}", .got, .expected)]
    UnexpectedClientIdentifier { got: String, expected: String },
}
