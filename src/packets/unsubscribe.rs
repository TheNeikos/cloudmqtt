//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::unsubscribe::UnsubscribeProperties,
    anker: "_Toc3901182",
    pub struct UnsubscribeProperties {
        (anker: "_Toc3901183")
        subscription_identifier: SubscriptionIdentifier with setter = u32,

        (anker: "_Toc3901183")
        user_properties: UserProperties<'i> with setter = crate::properties::UserProperty,
    }
}
