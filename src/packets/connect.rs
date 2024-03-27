//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::connect::ConnectProperties,
    anker: "_Toc3901046",
    pub struct ConnectProperties {
        (anker: "_Toc3901048")
        session_expiry_interval: SessionExpiryInterval with setter = u32,

        (anker: "_Toc3901049")
        receive_maximum: ReceiveMaximum with setter = u32,

        (anker: "_Toc3901050")
        maximum_packet_size: MaximumPacketSize with setter = u32,

        (anker: "_Toc3901051")
        topic_alias_maximum: TopicAliasMaximum with setter = u32,

        (anker: "_Toc3901052")
        request_response_information: RequestResponseInformation with setter = u8,

        (anker: "_Toc3901053")
        request_problem_information: RequestProblemInformation with setter = u8,

        (anker: "_Toc3901054")
        user_properties: UserProperties<'a> with setter = crate::properties::UserProperty,

        (anker: "_Toc3901055")
        authentication_method: AuthenticationMethod<'a> with setter = String,

        (anker: "_Toc3901056")
        authentication_data: AuthenticationData<'a> with setter = Vec<u8>,
    }
}

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::connect::ConnectWillProperties,
    anker: "_Toc3901060",
    pub struct ConnectWillProperties {
        (anker: "_Toc3901062")
        will_delay_interval: WillDelayInterval with setter = u16,

        (anker: "_Toc3901063")
        payload_format_indicator: PayloadFormatIndicator with setter = u8,

        (anker: "_Toc3901064")
        message_expiry_interval: MessageExpiryInterval with setter = u32,

        (anker: "_Toc3901065")
        content_type: ContentType<'i> with setter = String,

        (anker: "_Toc3901066")
        response_topic: ResponseTopic<'i> with setter = String,

        (anker: "_Toc3901067")
        correlation_data: CorrelationData<'i> with setter = Vec<u8>,

        (anker: "_Toc3901068")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}
