//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
#![cfg_attr(all(feature = "mqttv5", not(feature = "mqttv3"), not(test)), no_std)]
#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_types)]

#[cfg(feature = "mqttv3")]
pub mod v3;

/// MQTTv5 binary format parsing
///
#[doc = util::md_speclink!("_Toc3901000")]
#[cfg(feature = "mqttv5")]
pub mod v5;
