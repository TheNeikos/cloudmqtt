//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#![no_main]
use libfuzzer_sys::fuzz_target;
use mqtt_format::v3::packet::mpacket;
extern crate mqtt_format;

fuzz_target!(|data: &[u8]| {
    let _ = mpacket(data);
});
