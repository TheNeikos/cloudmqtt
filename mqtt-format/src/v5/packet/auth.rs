crate::v5::reason_code::make_combined_reason_code! {
    pub enum AuthReasonCode {
        ContinueAuthentication = crate::v5::reason_code::ContinueAuthentication,
        ReAuthenticate = crate::v5::reason_code::ReAuthenticate,
        Success = crate::v5::reason_code::Success,
    }
}
