//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::disconnect::DisconnectProperties,
    anker: "_Toc3901209",
    pub struct DisconnectProperties {
        (anker: "_Toc3901211")
        session_expiry_interval: SessionExpiryInterval with setter = u32,

        (anker: "_Toc3901212")
        reason_string: ReasonString<'i> with setter = String,

        (anker: "_Toc3901213")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,

        (anker: "_Toc3901214")
        server_reference: ServerReference<'i> with setter = String,
    }
}
