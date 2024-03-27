//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::pubcomp::PubcompProperties,
    anker: "_Toc3901153",
    pub struct PubcompProperties {
        (anker: "_Toc3901154")
        reason_string: ReasonString<'i> with setter = String,

        (anker: "_Toc3901155")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}
