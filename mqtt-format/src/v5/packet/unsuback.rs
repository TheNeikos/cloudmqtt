crate::v5::reason_code::make_combined_reason_code! {
    pub enum UnsubackReasonCode {
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NoSubscriptionExisted = crate::v5::reason_code::NoSubscriptionExisted,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        Success = crate::v5::reason_code::Success,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
    }
}
