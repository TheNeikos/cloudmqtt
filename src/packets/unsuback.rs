//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::unsuback::UnsubackProperties,
    anker: "_Toc3901190",
    pub struct UnsubackProperties {
        (anker: "_Toc3901192")
        reason_string: ReasonString<'i> with setter = String,

        (anker: "_Toc3901193")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}
