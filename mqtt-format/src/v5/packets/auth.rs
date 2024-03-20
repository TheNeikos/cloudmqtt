use winnow::Bytes;

use crate::v5::{
    variable_header::{AuthenticationData, AuthenticationMethod, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum AuthReasonCode {
        ContinueAuthentication = crate::v5::reason_code::ContinueAuthentication,
        ReAuthenticate = crate::v5::reason_code::ReAuthenticate,
        Success = crate::v5::reason_code::Success,
    }
}

crate::v5::properties::define_properties! {
    pub struct AuthProperties<'i> {
        authentication_method: AuthenticationMethod<'i>,
        authentication_data: AuthenticationData<'i>,
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
}

pub struct MAuth<'i> {
    pub reason: AuthReasonCode,
    pub properties: AuthProperties<'i>,
}

impl<'i> MAuth<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let reason = AuthReasonCode::parse(input)?;
        let properties = AuthProperties::parse(input)?;

        Ok(Self { reason, properties })
    }
}
