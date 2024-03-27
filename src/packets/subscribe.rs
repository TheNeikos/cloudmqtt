//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::subscribe::SubscribeProperties,
    anker: "_Toc3901164",
    pub struct SubscribeProperties {
        (anker: "_Toc3901166")
        subscription_identifier: SubscriptionIdentifier with setter = u32,

        (anker: "_Toc3901167")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}
