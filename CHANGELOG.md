# Changelog 'cloudmqtt'

## v0.5.0

The 0.5.0 public release of the cloudmqtt crate. It is still considered to be in an alpha state.

Changed:

- Upgraded to Rust version 1.85, and bumped edition requirement to 2024
- Rewrote the cloudmqtt crate and split it up into cloudmqtt-core which is no_std

## v0.4.0

The 0.4.0 public release of the cloudmqtt crate. It is still considered to be in an alpha state.

Changed:

- Removed requirement of the server to be behind an `Arc`. It can be used as is normally now.

## v0.3.1

The 0.3.1 public release of the cloudmqtt crate. It is still considered to be in an alpha state.

Added:

- Added method to access the String inside a ClientId

## v0.3.0

The 0.3.0 public release of the cloudmqtt crate. It is still considered to be in an alpha state.

Added:

- Added listening to messages to the server
- Added authentication provider to the server
- Added subscription acknowledgement to the server

## v0.2.0

The 0.2.0 public release of the cloudmqtt crate. It is still considered to be in an alpha state.

Added:

- Added an MQTT Server that can act as a broker
- Added QoS 1 and 2 levels to the broker

## v0.1.0

The 0.1.0 is the initial public release of the cloudmqtt crate. As such it is still in an alpha state.

Added:

- Added support for connecting via TCP to a MQTT server
- Added support for keep alive heartbeats
- Added support for QoS 1 and Qos 2 levels


