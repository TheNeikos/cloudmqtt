//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use mqtt_format::v3::packet::MConnect;
use mqtt_format::v3::packet::MPacket;

use crate::packet_invariant::PacketInvariant;
use crate::report::Report;
use crate::report::ReportResult;

pub struct NoUsernameMeansNoPassword;

impl PacketInvariant for NoUsernameMeansNoPassword {
    fn test_invariant(&self, packet: &MPacket<'_>) -> Option<miette::Result<Report>> {
        let result = if let MPacket::Connect(MConnect {
            username, password, ..
        }) = packet
        {
            if username.is_none() {
                if password.is_some() {
                    ReportResult::Failure
                } else {
                    ReportResult::Success
                }
            } else {
                ReportResult::Success
            }
        } else {
            return None;
        };

        Some(Ok(crate::mk_report! {
            name: "If the CONNECT packet flag for username is set, a username must be present",
            desc: "If the User Name Flag is set to 1, a user name MUST be present in the payload.",
            normative: "[MQTT-3.1.2-18, MQTT-3.1.2-19]",
            result
        }))
    }
}
