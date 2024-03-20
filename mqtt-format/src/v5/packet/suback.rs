crate::v5::reason_code::make_combined_reason_code! {
    pub enum SubackReasonCode {
        GrantedQoS0 = crate::v5::reason_code::GrantedQoS0,
        GrantedQoS1 = crate::v5::reason_code::GrantedQoS1,
        GrantedQoS2 = crate::v5::reason_code::GrantedQoS2,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        SharedSubscriptionsNotSupported = crate::v5::reason_code::SharedSubscriptionsNotSupported,
        SubscriptionIdentifiersNotSupported = crate::v5::reason_code::SubscriptionIdentifiersNotSupported,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        WildcardSubscriptionsNotSupported = crate::v5::reason_code::WildcardSubscriptionsNotSupported,
    }
}
