#![no_main]
use libfuzzer_sys::fuzz_target;
use mqtt_format::v3::packet::mpacket;
extern crate mqtt_format;

fuzz_target!(|data: &[u8]| {
    let _ = mpacket(data);
});
