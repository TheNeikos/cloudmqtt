use winnow::{
    error::{ErrMode, FromExternalError, InputError, ParserError},
    Bytes, Parser,
};

use crate::v5::{
    bytes::parse_binary_data,
    fixed_header::{PacketType, QualityOfService},
    integers::{parse_u16, parse_u32, parse_variable_u32},
    level::ProtocolLevel,
    strings::parse_string,
    variable_header::{
        AuthenticationData, AuthenticationMethod, ContentType, CorrelationData, MaximumPacketSize,
        MessageExpiryInterval, PayloadFormatIndicator, ReceiveMaximum, RequestProblemInformation,
        RequestResponseInformation, ResponseTopic, SessionExpiryInterval, TopicAliasMaximum,
        UserProperties, WillDelayInterval,
    },
    MResult,
};

pub struct MConnect<'i> {
    client_identifier: &'i str,
    username: Option<&'i str>,
    password: Option<&'i [u8]>,
    clean_start: bool,
    will: Option<Will<'i>>,
    properties: ConnectProperties<'i>,
}

crate::v5::properties::define_properties! {
    pub struct ConnectProperties<'i> {
        session_expiry_interval: SessionExpiryInterval,
        receive_maximum: ReceiveMaximum,
        maximum_packet_size: MaximumPacketSize,
        topic_alias_maximum: TopicAliasMaximum,
        request_response_information: RequestResponseInformation,
        request_problem_information: RequestProblemInformation,
        user_properties: UserProperties<'i>,
        authentication_method: AuthenticationMethod<'i>,
        authentication_data: AuthenticationData<'i>,
    }
}

impl<'i> MConnect<'i> {
    const PACKET_TYPE: PacketType = PacketType::Connect;

    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        // parse header
        let protocol_name = crate::v5::strings::parse_string(input)?;
        let protocol_level = ProtocolLevel::parse(input)?;
        let (_, clean_start, will_flag, will_qos, will_retain, password_flag, user_name_flag) =
            winnow::binary::bits::bits::<_, _, InputError<(_, usize)>, _, _>((
                winnow::binary::bits::pattern(0x0, 1usize),
                winnow::binary::bits::bool,
                winnow::binary::bits::bool,
                winnow::binary::bits::take(2usize)
                    .try_map(<QualityOfService as TryFrom<u8>>::try_from),
                winnow::binary::bits::bool,
                winnow::binary::bits::bool,
                winnow::binary::bits::bool,
            ))
            .parse_next(input)
            .map_err(|_: ErrMode<InputError<_>>| {
                ErrMode::from_error_kind(input, winnow::error::ErrorKind::Slice)
            })?;

        let keep_alive = parse_u16(input)?;

        let properties = ConnectProperties::parse(input)?;

        // finished parsing header, now parse payload

        let client_identifier = {
            let client_identifier = parse_string(input)?;
            if client_identifier.len() == 0 {
                // Generate client ID?
            }
            client_identifier
        };

        let will = will_flag
            .then(|| {
                let properties = ConnectWillProperties::parse(input)?;
                let topic = parse_string(input)?;
                let payload = crate::v5::bytes::parse_binary_data(input)?;

                Ok(Will {
                    properties,
                    topic,
                    payload,
                })
            })
            .transpose()?;

        let username = user_name_flag.then(|| parse_string(input)).transpose()?;
        let password = password_flag
            .then(|| parse_binary_data(input))
            .transpose()?;

        Ok(Self {
            client_identifier,
            username,
            password,
            will,
            clean_start,
            properties,
        })
    }
}

pub struct Will<'i> {
    properties: ConnectWillProperties<'i>,
    topic: &'i str,
    payload: &'i [u8],
}

crate::v5::properties::define_properties! {
    pub struct ConnectWillProperties<'i> {
        will_delay_interval: WillDelayInterval,
        payload_format_indicator: PayloadFormatIndicator,
        message_expiry_interval: MessageExpiryInterval,
        content_type: ContentType<'i>,
        response_topic: ResponseTopic<'i>,
        correlation_data: CorrelationData<'i>,
    }
}
