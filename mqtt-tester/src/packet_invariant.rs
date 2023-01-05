//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v3::packet::MPacket;

use crate::report::Report;

pub trait PacketInvariant {
    fn test_invariant(&self, packet: &MPacket<'_>) -> miette::Result<Report>;
}
