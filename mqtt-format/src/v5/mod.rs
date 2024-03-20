//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#![deny(missing_debug_implementations)]
#![deny(clippy::std_instead_of_core)]
#![deny(clippy::alloc_instead_of_core)]

//! MQTTv5 binary format parsing
//!
#![doc = util::md_speclink!("_Toc3901000")]
//!
//! The main entry point of the v5 module is found in [packets::MqttPacket] and its associated
//! [packets::MqttPacket::parse_complete] method.
//!
//! This allows to retrieve a zero-copy deserialized form of a single MQTTPacket.
//! All protocol-level invariants are checked here. Nonetheless, dynamic protocol violations cannot
//! be detected at this level, and as such fall onto the responsibility of the user.
//!
//! # Example
//!
//! ```rust
//! use mqtt_format::v5::packets::MqttPacket;
//!# // A PINGREQ packet
//!# fn read_input() -> &'static [u8] { &[0b1100_0000,  0x0] }
//! let input: &[u8] = read_input();
//!
//! let packet = MqttPacket::parse_complete(input).expect("A valid MQTT Packet");
//!
//! match packet {
//!     MqttPacket::Pingreq(_) => println!("Got a PINGREQ!"),
//!     packet => panic!("Got an unexpected packet: {packet:?}"),
//! }
//! ```

pub mod bytes;
pub mod fixed_header;
pub mod integers;
pub mod packets;
pub mod properties;
pub mod reason_code;
pub mod strings;
mod util;
pub mod variable_header;

pub type MResult<O> = winnow::PResult<O>;
