//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v3::packet::MConnect;
use mqtt_format::v3::packet::MPacket;
use mqtt_format::v3::strings::MString;

use crate::packet_invariant::PacketInvariant;
use crate::report::Report;
use crate::report::ReportResult;

pub struct ConnectPacketProtocolName;

impl PacketInvariant for ConnectPacketProtocolName {
    fn test_invariant(&self, packet: &MPacket<'_>) -> Option<miette::Result<Report>> {
        let result = if let MPacket::Connect(MConnect { protocol_name, .. }) = packet {
            if *protocol_name == (MString { value: "MQTT" }) {
                ReportResult::Success
            } else {
                ReportResult::Inconclusive
            }
        } else {
            return None;
        };

        Some(Ok(crate::mk_report! {
            name: "The CONNECT packet must have protocol_name = 'MQTT'",
            desc: "The Protocol Name is a UTF-8 encoded string that represents the protocol name “MQTT”, capitalized as shown",
            normative: "[MQTT-3.1.2-1]",
            result
        }))
    }
}
