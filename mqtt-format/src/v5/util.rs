//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

macro_rules! speclink {
    ($anker:literal) => {
        core::concat!(
            "https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#",
            $anker
        )
    };
}
pub(crate) use speclink;

macro_rules! md_speclink {
    ($anker:literal) => {
        core::concat!(
            "[📖 Specification](",
            $crate::v5::util::speclink!($anker),
            ")"
        )
    };
}
pub(crate) use md_speclink;
