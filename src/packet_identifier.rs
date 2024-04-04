//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PacketIdentifierNonZero(std::num::NonZeroU16);

impl PacketIdentifierNonZero {
    #[inline]
    pub fn get(&self) -> u16 {
        self.0.get()
    }
}

impl std::fmt::Display for PacketIdentifierNonZero {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
        mqtt_format::v5::variable_header::PacketIdentifier(value.0)
    }
}

impl From<std::num::NonZeroU16> for PacketIdentifierNonZero {
    fn from(value: std::num::NonZeroU16) -> Self {
        Self(value)
    }
}
