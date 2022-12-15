# MQTT Format

A pure Rust MQTT packet parsing/serializing library 

`mqtt_format` allows users to get zero-copy and fast parsing and serializing of
MQTT packets. Both v3 and v5 is supported.

This crate supports the [`yoke`](https://docs.rs/yoke/latest/yoke/) library,
which can be enabled with the `yoke` feature.
