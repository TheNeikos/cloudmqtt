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
    packet_type: MAuth,
    anker: "_Toc3901221",
    pub struct AuthProperties<'i> {
        (anker: "_Toc3901223")
        authentication_method: AuthenticationMethod<'i>,

        (anker: "_Toc3901224")
        authentication_data: AuthenticationData<'i>,

        (anker: "_Toc3901225")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901226")
        user_properties: UserProperties<'i>,
    }
}

#[doc = crate::v5::util::md_speclink!("_Toc3901217")]
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
