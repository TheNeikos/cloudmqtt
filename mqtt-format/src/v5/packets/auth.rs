//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::variable_header::AuthenticationData;
use crate::v5::variable_header::AuthenticationMethod;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

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

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901217")]
pub struct MAuth<'i> {
    pub reason: AuthReasonCode,
    pub properties: AuthProperties<'i>,
}

impl<'i> MAuth<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MAuth", |input: &mut &'i Bytes| {
            let reason = AuthReasonCode::parse(input)?;
            let properties = AuthProperties::parse(input)?;

            Ok(Self { reason, properties })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.reason.binary_size() + self.properties.binary_size()
    }

    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        self.reason.write(buffer).await?;
        self.properties.write(buffer).await
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::auth::AuthProperties;
    use crate::v5::packets::auth::AuthReasonCode;
    use crate::v5::packets::auth::MAuth;
    use crate::v5::variable_header::AuthenticationData;
    use crate::v5::variable_header::AuthenticationMethod;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::UserProperties;

    #[tokio::test]
    async fn test_roundtrip_mauth_no_props() {
        crate::v5::test::make_roundtrip_test!(MAuth {
            reason: AuthReasonCode::ContinueAuthentication,
            properties: AuthProperties {
                authentication_method: None,
                authentication_data: None,
                reason_string: None,
                user_properties: None,
            },
        });
    }

    #[tokio::test]
    async fn test_roundtrip_mauth_props() {
        crate::v5::test::make_roundtrip_test!(MAuth {
            reason: AuthReasonCode::ContinueAuthentication,
            properties: AuthProperties {
                authentication_method: Some(AuthenticationMethod("foo")),
                authentication_data: Some(AuthenticationData(&[0, 1])),
                reason_string: Some(ReasonString("foo")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            },
        });
    }
}
