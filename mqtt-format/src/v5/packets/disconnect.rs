use winnow::{Bytes, Parser};

use crate::v5::{
    properties::define_properties,
    variable_header::{ReasonString, ServerReference, SessionExpiryInterval, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum DisconnectReasonCode {
        AdministrativeAction = crate::v5::reason_code::AdministrativeAction,
        BadAuthenticationMethod = crate::v5::reason_code::BadAuthenticationMethod,
        ConnectionRateExceeded = crate::v5::reason_code::ConnectionRateExceeded,
        DisconnectWithWillMessage = crate::v5::reason_code::DisconnectWithWillMessage,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        KeepAliveTimeout = crate::v5::reason_code::KeepAliveTimeout,
        MalformedPacket = crate::v5::reason_code::MalformedPacket,
        MaximumConnectTime = crate::v5::reason_code::MaximumConnectTime,
        MessageRateTooHigh = crate::v5::reason_code::MessageRateTooHigh,
        NormalDisconnection = crate::v5::reason_code::NormalDisconnection,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketTooLarge = crate::v5::reason_code::PacketTooLarge,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        ProtocolError = crate::v5::reason_code::ProtocolError,
        QoSNotSupported = crate::v5::reason_code::QoSNotSupported,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        ReceiveMaximumExceeded = crate::v5::reason_code::ReceiveMaximumExceeded,
        RetainNotSupported = crate::v5::reason_code::RetainNotSupported,
        ServerBusy = crate::v5::reason_code::ServerBusy,
        ServerMoved = crate::v5::reason_code::ServerMoved,
        ServerShuttingDown = crate::v5::reason_code::ServerShuttingDown,
        SessionTakenOver = crate::v5::reason_code::SessionTakenOver,
        SharedSubscriptionsNotSupported = crate::v5::reason_code::SharedSubscriptionsNotSupported,
        SubscriptionIdentifiersNotSupported = crate::v5::reason_code::SubscriptionIdentifiersNotSupported,
        TopicAliasInvalid = crate::v5::reason_code::TopicAliasInvalid,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        UseAnotherServer = crate::v5::reason_code::UseAnotherServer,
        WildcardSubscriptionsNotSupported = crate::v5::reason_code::WildcardSubscriptionsNotSupported,
    }
}

define_properties! {
    pub struct DisconnectProperties<'i> {
        session_expiry_interval: SessionExpiryInterval,
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
        server_reference: ServerReference<'i>
    }
}

pub struct MDisconnect<'i> {
    reason_code: DisconnectReasonCode,
    properties: DisconnectProperties<'i>,
}

impl<'i> MDisconnect<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MDisconnect<'i>> {
        let (reason_code, properties) =
            (DisconnectReasonCode::parse, DisconnectProperties::parse).parse_next(input)?;

        Ok(MDisconnect {
            reason_code,
            properties,
        })
    }
}
