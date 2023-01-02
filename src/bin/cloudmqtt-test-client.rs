//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::process::exit;

use cloudmqtt::{client::MqttClient, client::MqttConnectionParams};
use futures::StreamExt;
use mqtt_format::v3::will::MLastWill;

fn print_error_and_quit(e: &str) -> ! {
    eprintln!("{}", e);
    exit(1);
}

#[tokio::main]
async fn main() {
    let (client_duplex, server_duplex) = tokio::io::duplex(512);

    let (mut read_dup, mut write_dup) = tokio::io::split(server_duplex);

    tokio::spawn(async move { tokio::io::copy(&mut tokio::io::stdin(), &mut write_dup).await });
    tokio::spawn(async move { tokio::io::copy(&mut read_dup, &mut tokio::io::stdout()).await });

    let client = match MqttClient::connect_v3_duplex(
        client_duplex,
        MqttConnectionParams {
            clean_session: false,
            will: Some(MLastWill {
                topic: mqtt_format::v3::strings::MString {
                    value: "hello/world",
                },
                payload: b"I died!",
                qos: mqtt_format::v3::qos::MQualityOfService::AtMostOnce,
                retain: false,
            }),
            username: None,
            password: None,
            keep_alive: 5,
            client_id: mqtt_format::v3::strings::MString {
                value: "mqtt-client-test",
            },
        },
    )
    .await
    {
        Ok(client) => client,
        Err(e) => print_error_and_quit(&format!("Could not connect: {e}")),
    };

    tokio::spawn(client.hearbeat(None));

    let packet_stream = client.build_packet_stream().build();
    let mut packet_stream = Box::pin(packet_stream.stream());

    loop {
        let packet = match packet_stream.next().await {
            Some(Ok(packet)) => packet,
            None => {
                eprintln!("Stream ended, stopping");
                break;
            }
            Some(Err(error)) => print_error_and_quit(&format!("Stream errored: {error}")),
        };
    }
}
