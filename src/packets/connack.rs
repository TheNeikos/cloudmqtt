//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::connack::ConnackProperties,
    anker: "_Toc3901080",
    pub struct ConnackProperties {
        (anker: "_Toc3901082")
        session_expiry_interval: SessionExpiryInterval with setter = u32,

        (anker: "_Toc3901083")
        receive_maximum: ReceiveMaximum with setter = u32,

        (anker: "_Toc3901084")
        maximum_qos: MaximumQoS with setter = u8,

        (anker: "_Toc3901085")
        retain_available: RetainAvailable with setter = u8,

        (anker: "_Toc3901086")
        maximum_packet_size: MaximumPacketSize with setter = u32,

        (anker: "_Toc3901087")
        assigned_client_identifier: AssignedClientIdentifier<'i> with setter = String,

        (anker: "_Toc3901088")
        topic_alias_maximum: TopicAliasMaximum with setter = u16,

        (anker: "_Toc3901089")
        reason_string: ReasonString<'i> with setter = String,

        (anker: "_Toc3901090")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,

        (anker: "_Toc3901091")
        wildcard_subscription_available: WildcardSubscriptionAvailable with setter = u8,

        (anker: "_Toc3901092")
        subscription_identifiers_available: SubscriptionIdentifiersAvailable with setter = u8,

        (anker: "_Toc3901093")
        shared_scubscription_available: SharedSubscriptionAvailable with setter = u8,

        (anker: "_Toc3901094")
        server_keep_alive: ServerKeepAlive with setter = u16,

        (anker: "_Toc3901095")
        response_information: ResponseInformation<'i> with setter = String,

        (anker: "_Toc3901096")
        server_reference: ServerReference<'i> with setter = String,

        (anker: "_Toc3901097")
        authentication_method: AuthenticationMethod<'i> with setter = String,

        (anker: "_Toc3901098")
        authentication_data: AuthenticationData<'i> with setter = Vec<u8>,
    }
}
