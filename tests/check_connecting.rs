//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::pin::Pin;

use bytes::Bytes;
use cloudmqtt::{client::MqttClient, client::MqttConnectionParams, error::MqttError};
use mqtt_format::v3::{connect_return::MConnectReturnCode, packet::MPacket, strings::MString};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn check_simple_connect() {
    let (mut duplex, client_duplex) = tokio::io::duplex(512);

    let client = MqttClient::connect_v3_duplex(
        client_duplex,
        MqttConnectionParams {
            clean_session: false,
            will: None,
            username: None,
            password: None,
            keep_alive: 16,
            client_id: MString {
                value: "check_simple_connect",
            },
        },
    );

    let response = MPacket::Connack {
        session_present: false,
        connect_return_code: mqtt_format::v3::connect_return::MConnectReturnCode::Accepted,
    };

    let mut buf = Vec::new();
    response
        .write_to(Pin::new(&mut buf))
        .await
        .expect("to create response CONNACK");

    duplex
        .write_all_buf(&mut Bytes::from(buf))
        .await
        .expect("to write response CONNACK");

    client.await.unwrap();
}

#[tokio::test]
async fn check_invalid_connect() {
    let (mut duplex, client_duplex) = tokio::io::duplex(512);

    let client = MqttClient::connect_v3_duplex(
        client_duplex,
        MqttConnectionParams {
            clean_session: false,
            will: None,
            username: None,
            password: None,
            keep_alive: 16,
            client_id: MString {
                value: "check_invalid_connect",
            },
        },
    );

    let response = MPacket::Connack {
        session_present: false,
        connect_return_code: mqtt_format::v3::connect_return::MConnectReturnCode::NotAuthorized,
    };

    let mut buf = Vec::new();
    response
        .write_to(Pin::new(&mut buf))
        .await
        .expect("to create response CONNACK");

    duplex
        .write_all_buf(&mut Bytes::from(buf))
        .await
        .expect("to write response CONNACK");

    match client.await {
        Err(MqttError::ConnectionRejected(MConnectReturnCode::NotAuthorized)) => {
            // We're good
        }
        Ok(_) => panic!("Should not have succeeded"),
        Err(e) => panic!("Should have errored: {}", e),
    }
}
