//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::stream::Stream;
use winnow::Bytes;

use crate::v5::fixed_header::QualityOfService;
use crate::v5::variable_header::ContentType;
use crate::v5::variable_header::CorrelationData;
use crate::v5::variable_header::MessageExpiryInterval;
use crate::v5::variable_header::PayloadFormatIndicator;
use crate::v5::variable_header::ResponseTopic;
use crate::v5::variable_header::SubscriptionIdentifier;
use crate::v5::variable_header::TopicAlias;
use crate::v5::variable_header::UserProperties;
use crate::v5::MResult;

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901100")]
pub struct MPublish<'i> {
    pub duplicate: bool,
    pub quality_of_service: QualityOfService,
    pub retain: bool,
    pub topic_name: &'i str,
    pub packet_identifier: crate::v5::variable_header::PacketIdentifier,
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
        let topic_name = crate::v5::strings::parse_string(input)?;
        if !sanity_check_topic_name(topic_name) {
            return Err(ErrMode::from_error_kind(
                input,
                winnow::error::ErrorKind::Verify,
            ));
        }

        let packet_identifier = crate::v5::variable_header::PacketIdentifier::parse(input)?;
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
    }
}

fn sanity_check_topic_name(topic_name: &str) -> bool {
    topic_name.chars().all(|c| c != '#' && c != '*')
}
