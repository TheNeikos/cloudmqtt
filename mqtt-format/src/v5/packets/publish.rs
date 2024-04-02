//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::stream::Stream;
use winnow::Bytes;
use winnow::Parser;

use crate::v5::qos::QualityOfService;
use crate::v5::strings::write_string;
use crate::v5::variable_header::ContentType;
use crate::v5::variable_header::CorrelationData;
use crate::v5::variable_header::MessageExpiryInterval;
use crate::v5::variable_header::PayloadFormatIndicator;
use crate::v5::variable_header::ResponseTopic;
use crate::v5::variable_header::SubscriptionIdentifier;
use crate::v5::variable_header::TopicAlias;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::MqttWriteError;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901100")]
pub struct MPublish<'i> {
    pub duplicate: bool,
    pub quality_of_service: QualityOfService,
    pub retain: bool,
    pub topic_name: &'i str,
    pub packet_identifier: Option<crate::v5::variable_header::PacketIdentifier>,
    pub properties: PublishProperties<'i>,
    pub payload: &'i [u8],
}

crate::v5::properties::define_properties! {
    packet_type: MPublish,
    anker: "_Toc3901109",
    pub struct PublishProperties<'i> {
        (anker: "_Toc3901111")
        payload_format_indicator: PayloadFormatIndicator,

        (anker: "_Toc3901112")
        message_expiry_interval: MessageExpiryInterval,

        (anker: "_Toc3901113")
        topic_alias: TopicAlias,

        (anker: "_Toc3901114")
        response_topic: ResponseTopic<'i>,

        (anker: "_Toc3901115")
        correlation_data: CorrelationData<'i>,

        (anker: "_Toc3901116")
        user_properties: UserProperties<'i>,

        (anker: "_Toc3901117")
        subscription_identifier: SubscriptionIdentifier,

        (anker: "_Toc3901118")
        content_type: ContentType<'i>,
    }
}

impl<'i> MPublish<'i> {
    pub fn parse(
        duplicate: bool,
        quality_of_service: QualityOfService,
        retain: bool,
        input: &mut &'i Bytes,
    ) -> MResult<Self> {
        winnow::combinator::trace("MPublish", |input: &mut &'i Bytes| {
            let topic_name = crate::v5::strings::parse_string(input)?;
            if !sanity_check_topic_name(topic_name) {
                return Err(ErrMode::from_error_kind(
                    input,
                    winnow::error::ErrorKind::Verify,
                ));
            }

            let packet_identifier = if quality_of_service != QualityOfService::AtMostOnce {
                Some(crate::v5::variable_header::PacketIdentifier::parse(input)?)
            } else {
                None
            };
            let properties = PublishProperties::parse(input)?;

            let payload = input.finish();

            Ok(Self {
                duplicate,
                quality_of_service,
                retain,
                topic_name,
                packet_identifier,
                properties,
                payload,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        crate::v5::strings::string_binary_size(self.topic_name)
            + self
                .packet_identifier
                .map(|pi| pi.binary_size())
                .unwrap_or(0)
            + self.properties.binary_size()
            + crate::v5::bytes::binary_data_binary_size(self.payload)
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        write_string(buffer, self.topic_name)?;

        if self.quality_of_service == QualityOfService::AtMostOnce
            && self.packet_identifier.is_some()
        {
            return Err(MqttWriteError::Invariant.into());
        }
        if let Some(pi) = self.packet_identifier {
            pi.write(buffer)?;
        }

        self.properties.write(buffer)?;

        buffer.write_slice(self.payload)
    }
}

fn sanity_check_topic_name(topic_name: &str) -> bool {
    topic_name.chars().all(|c| c != '#' && c != '*')
}

#[cfg(test)]
mod test {
    use crate::v5::packets::publish::MPublish;
    use crate::v5::packets::publish::PublishProperties;
    use crate::v5::qos::QualityOfService;
    use crate::v5::variable_header::PacketIdentifier;

    #[test]
    fn test_roundtrip_pubcomp_no_props() {
        let mut writer = crate::v5::test::TestWriter { buffer: Vec::new() };

        let duplicate = true;
        let quality_of_service = QualityOfService::ExactlyOnce;
        let retain = false;

        let instance = MPublish {
            duplicate,
            quality_of_service,
            retain,
            topic_name: "top/ic",
            packet_identifier: Some(PacketIdentifier(1)),
            properties: PublishProperties {
                payload_format_indicator: None,
                message_expiry_interval: None,
                topic_alias: None,
                response_topic: None,
                correlation_data: None,
                user_properties: None,
                subscription_identifier: None,
                content_type: None,
            },
            payload: &[0x12, 0x34],
        };
        instance.write(&mut writer).unwrap();
        let output = MPublish::parse(
            duplicate,
            quality_of_service,
            retain,
            &mut winnow::Bytes::new(&writer.buffer),
        )
        .unwrap();
        assert_eq!(instance, output);
    }

    #[test]
    fn test_roundtrip_puback_with_props() {
        let mut writer = crate::v5::test::TestWriter { buffer: Vec::new() };
        let duplicate = true;
        let quality_of_service = QualityOfService::ExactlyOnce;
        let retain = false;

        let instance = MPublish {
            duplicate,
            quality_of_service,
            retain,
            topic_name: "top/ic",
            packet_identifier: Some(PacketIdentifier(1)),
            properties: PublishProperties {
                payload_format_indicator: None,
                message_expiry_interval: None,
                topic_alias: None,
                response_topic: None,
                correlation_data: None,
                user_properties: None,
                subscription_identifier: None,
                content_type: None,
            },
            payload: &[0x12, 0x34],
        };
        instance.write(&mut writer).unwrap();
        let output = MPublish::parse(
            duplicate,
            quality_of_service,
            retain,
            &mut winnow::Bytes::new(&writer.buffer),
        )
        .unwrap();
        assert_eq!(instance, output);
    }
}
