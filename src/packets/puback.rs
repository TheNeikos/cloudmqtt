//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use yoke::Yoke;

use super::MqttPacket;
use super::StableBytes;

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::puback::PubackProperties,
    anker: "_Toc3901125",
    pub struct PubackProperties {
        (anker: "_Toc3901127")
        reason_string: ReasonString<'i> with setter = String,

        (anker: "_Toc3901128")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}

#[derive(Clone, Debug)]
pub struct Puback {
    packet: Yoke<mqtt_format::v5::packets::puback::MPuback<'static>, StableBytes>,
}

impl Puback {
    pub(crate) fn get(&self) -> &mqtt_format::v5::packets::puback::MPuback<'_> {
        self.packet.get()
    }
}

impl TryFrom<MqttPacket> for Puback {
    type Error = ();

    fn try_from(value: MqttPacket) -> Result<Self, Self::Error> {
        let packet = value.packet.try_map_project(|p, _| match p {
            mqtt_format::v5::packets::MqttPacket::Puback(puback) => Ok(puback),
            _ => Err(()),
        })?;

        Ok(Puback { packet })
    }
}
