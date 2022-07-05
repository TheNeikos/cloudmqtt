//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use super::{qos::MQualityOfService, strings::MString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MLastWill<'message> {
    pub topic: MString<'message>,
    pub payload: &'message [u8],
    pub qos: MQualityOfService,
    pub retain: bool,
}

impl<'message> MLastWill<'message> {
    pub fn get_len(&self) -> usize {
        MString::get_len(&self.topic) + (2 + self.payload.len())
    }
}
