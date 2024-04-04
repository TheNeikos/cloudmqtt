//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PacketIdentifier(u16);

impl From<mqtt_format::v5::variable_header::PacketIdentifier> for PacketIdentifier {
    fn from(value: mqtt_format::v5::variable_header::PacketIdentifier) -> Self {
        Self(value.0)
    }
}

impl From<PacketIdentifier> for mqtt_format::v5::variable_header::PacketIdentifier {
    fn from(value: PacketIdentifier) -> mqtt_format::v5::variable_header::PacketIdentifier {
        mqtt_format::v5::variable_header::PacketIdentifier(value.0)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PacketIdentifierNonZero(std::num::NonZeroU16);

impl TryFrom<mqtt_format::v5::variable_header::PacketIdentifier> for PacketIdentifierNonZero {
    type Error = (); // TODO

    fn try_from(
        value: mqtt_format::v5::variable_header::PacketIdentifier,
    ) -> Result<Self, Self::Error> {
        std::num::NonZeroU16::try_from(value.0)
            .map(Self)
            .map_err(drop) // TODO
    }
}

impl From<PacketIdentifierNonZero> for mqtt_format::v5::variable_header::PacketIdentifier {
    fn from(value: PacketIdentifierNonZero) -> mqtt_format::v5::variable_header::PacketIdentifier {
        mqtt_format::v5::variable_header::PacketIdentifier(value.0.get())
    }
}
