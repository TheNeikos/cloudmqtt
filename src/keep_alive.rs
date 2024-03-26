//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::num::NonZeroU16;
use std::time::Duration;

pub enum KeepAlive {
    Disabled,
    Seconds(NonZeroU16),
}

impl KeepAlive {
    pub(crate) fn as_u16(&self) -> u16 {
        match self {
            KeepAlive::Disabled => 0,
            KeepAlive::Seconds(s) => s.get(),
        }
    }
}
impl TryFrom<Duration> for KeepAlive {
    type Error = ();
    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        todo!()
    }
}
