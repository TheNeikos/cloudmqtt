use winnow::{
    error::{ErrMode, ParserError},
    stream::Stream,
    Bytes,
};

use crate::v5::{
    variable_header::{
        ContentType, CorrelationData, MessageExpiryInterval, PayloadFormatIndicator, ResponseTopic,
        SubscriptionIdentifier, TopicAlias, UserProperties,
    },
    MResult,
};

pub struct MPublish<'i> {
    pub topic_name: &'i str,
    pub packet_identifier: crate::v5::variable_header::PacketIdentifier,
    pub properties: PublishProperties<'i>,
    pub payload: &'i [u8],
}

crate::v5::properties::define_properties! {
    pub struct PublishProperties<'i> {
        payload_format_indicator: PayloadFormatIndicator,
        message_expiry_interval: MessageExpiryInterval,
        topic_alias: TopicAlias,
        response_topic: ResponseTopic<'i>,
        correlation_data: CorrelationData<'i>,
        user_properties: UserProperties<'i>,
        subscription_identifier: SubscriptionIdentifier,
        content_type: ContentType<'i>,
    }
}

impl<'i> MPublish<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
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
