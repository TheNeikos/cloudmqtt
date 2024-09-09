//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use yoke::Yoke;

use super::{MqttPacket, StableBytes};

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::publish::PublishProperties,
    anker: "_Toc3901109",
    pub struct PublishProperties {
        (anker: "_Toc3901111")
        payload_format_indicator: PayloadFormatIndicator with setter = u8,

        (anker: "_Toc3901112")
        message_expiry_interval: MessageExpiryInterval with setter = u32,

        (anker: "_Toc3901113")
        topic_alias: TopicAlias with setter = core::num::NonZeroU16,

        (anker: "_Toc3901114")
        response_topic: ResponseTopic<'i> with setter = String,

        (anker: "_Toc3901115")
        correlation_data: CorrelationData<'i> with setter = Vec<u8>,

        (anker: "_Toc3901116")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,

        (anker: "_Toc3901117")
        subscription_identifier: SubscriptionIdentifier with setter = u32,

        (anker: "_Toc3901118")
        content_type: ContentType<'i> with setter = String,
    }
}

#[derive(Clone, Debug)]
pub struct Publish {
    packet: Yoke<mqtt_format::v5::packets::publish::MPublish<'static>, StableBytes>,
}

impl Publish {
    pub(crate) fn get(&self) -> &mqtt_format::v5::packets::publish::MPublish<'_> {
        self.packet.get()
    }
}

impl TryFrom<MqttPacket> for Publish {
    type Error = ();

    fn try_from(value: MqttPacket) -> Result<Self, Self::Error> {
        let packet = value.packet.try_map_project(|p, _| match p {
            mqtt_format::v5::packets::MqttPacket::Publish(puback) => Ok(puback),
            _ => Err(()),
        })?;

        Ok(Publish { packet })
    }
}
