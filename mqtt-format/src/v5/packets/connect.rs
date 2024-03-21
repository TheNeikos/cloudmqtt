//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ErrMode;
use winnow::error::InputError;
use winnow::error::ParserError;
use winnow::Bytes;
use winnow::Parser;

use crate::v5::bytes::parse_binary_data;
use crate::v5::fixed_header::QualityOfService;
use crate::v5::integers::parse_u16;
use crate::v5::strings::parse_string;
use crate::v5::variable_header::AuthenticationData;
use crate::v5::variable_header::AuthenticationMethod;
use crate::v5::variable_header::ContentType;
use crate::v5::variable_header::CorrelationData;
use crate::v5::variable_header::MaximumPacketSize;
use crate::v5::variable_header::MessageExpiryInterval;
use crate::v5::variable_header::PayloadFormatIndicator;
use crate::v5::variable_header::ReceiveMaximum;
use crate::v5::variable_header::RequestProblemInformation;
use crate::v5::variable_header::RequestResponseInformation;
use crate::v5::variable_header::ResponseTopic;
use crate::v5::variable_header::SessionExpiryInterval;
use crate::v5::variable_header::TopicAliasMaximum;
use crate::v5::variable_header::UserProperties;
use crate::v5::variable_header::WillDelayInterval;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

#[derive(Debug)]
pub struct MConnect<'i> {
    pub client_identifier: &'i str,
    pub username: Option<&'i str>,
    pub password: Option<&'i [u8]>,
    pub clean_start: bool,
    pub will: Option<Will<'i>>,
    pub properties: ConnectProperties<'i>,
    pub keep_alive: u16,
}

crate::v5::properties::define_properties! {
    packet_type: MConnect,
    anker: "_Toc3901046",
    pub struct ConnectProperties<'i> {
        (anker: "_Toc3901048")
        session_expiry_interval: SessionExpiryInterval,

        (anker: "_Toc3901049")
        receive_maximum: ReceiveMaximum,

        (anker: "_Toc3901050")
        maximum_packet_size: MaximumPacketSize,

        (anker: "_Toc3901051")
        topic_alias_maximum: TopicAliasMaximum,

        (anker: "_Toc3901052")
        request_response_information: RequestResponseInformation,

        (anker: "_Toc3901053")
        request_problem_information: RequestProblemInformation,

        (anker: "_Toc3901054")
        user_properties: UserProperties<'i>,

        (anker: "_Toc3901055")
        authentication_method: AuthenticationMethod<'i>,

        (anker: "_Toc3901056")
        authentication_data: AuthenticationData<'i>,
    }
}

#[doc = crate::v5::util::md_speclink!("_Toc3901033")]
impl<'i> MConnect<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MConnect", |input: &mut &'i Bytes| {
            // parse header

            // verify the protocol name
            let _ = crate::v5::strings::parse_string
                .verify(|s: &str| s == "MQTT")
                .parse_next(input)?;

            // just for verification
            let _ = ProtocolLevel::parse
                .verify(|lvl: &ProtocolLevel| *lvl == ProtocolLevel::V5)
                .parse_next(input)?;

            let (
                _reserved,
                clean_start,
                will_flag,
                will_qos,
                will_retain,
                password_flag,
                user_name_flag,
            ) = winnow::binary::bits::bits::<_, _, InputError<(_, usize)>, _, _>((
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
                if client_identifier.is_empty() {
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
                        will_qos,
                        will_retain,
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
                keep_alive,
            })
        })
        .parse_next(input)
    }

    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        crate::v5::strings::write_string(buffer, "MQTT").await?;
        ProtocolLevel::V5.write(buffer).await?;

        let flags = {
            let reserved = 0;
            let clean_start = (self.clean_start as u8) << 1;
            let will = {
                if let Some(will) = self.will.as_ref() {
                    let will_flag = 1 << 2;
                    let will_qos = {
                        let qos: u8 = will.will_qos.into();
                        qos << 3
                    };
                    let will_retain = (will.will_retain as u8) << 5;
                    will_flag | will_qos | will_retain
                } else {
                    0
                }
            };
            let password = (self.password.is_some() as u8) << 6;
            let username = (self.username.is_some() as u8) << 7;

            reserved | clean_start | will | password | username
        };

        buffer.write_byte(flags).await?;
        buffer.write_u16(self.keep_alive).await?;
        self.properties.write(buffer).await?;
        crate::v5::strings::write_string(buffer, self.client_identifier).await?;
        if let Some(will) = self.will.as_ref() {
            will.write(buffer).await?;
        }
        if let Some(username) = self.username.as_ref() {
            crate::v5::strings::write_string(buffer, username).await?;
        }
        if let Some(password) = self.password.as_ref() {
            crate::v5::bytes::write_binary_data(buffer, password).await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Will<'i> {
    pub properties: ConnectWillProperties<'i>,
    pub topic: &'i str,
    pub payload: &'i [u8],
    pub will_qos: QualityOfService,
    pub will_retain: bool,
}

impl<'i> Will<'i> {
    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        self.properties.write(buffer).await?;
        crate::v5::strings::write_string(buffer, self.topic).await?;
        crate::v5::bytes::write_binary_data(buffer, self.payload).await?;
        Ok(())
    }
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ProtocolLevel {
    V3,
    V5,
}

impl ProtocolLevel {
    pub fn parse(input: &mut &Bytes) -> MResult<Self> {
        match winnow::binary::u8(input)? {
            3 => Ok(Self::V3),
            5 => Ok(Self::V5),
            _ => Err(ErrMode::from_error_kind(
                input,
                winnow::error::ErrorKind::Verify,
            )),
        }
    }

    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        match self {
            ProtocolLevel::V3 => buffer.write_byte(3).await,
            ProtocolLevel::V5 => buffer.write_byte(5).await,
        }
    }
}
