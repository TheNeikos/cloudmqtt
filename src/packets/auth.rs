//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::auth::AuthProperties,
    anker: "_Toc3901221",
    pub struct AuthProperties {
        (anker: "_Toc3901223")
        authentication_method: AuthenticationMethod<'a> with setter = String,

        (anker: "_Toc3901224")
        authentication_data: AuthenticationData<'a> with setter = Vec<u8>,

        (anker: "_Toc3901225")
        reason_string: ReasonString<'a> with setter = String,

        (anker: "_Toc3901226")
        user_properties: UserProperties<'a> with setter = crate::properties::UserProperty,
    }
}
