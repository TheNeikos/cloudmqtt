//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::process::exit;

use clap::Parser;
use cloudmqtt::{client::MqttClient, client::MqttConnectionParams};
use futures::StreamExt;
use mqtt_format::v3::{
    packet::{MPacket, MPublish},
    qos::MQualityOfService,
    strings::MString,
    subscription_request::MSubscriptionRequest,
    will::MLastWill,
};

fn print_error_and_quit(e: String) -> ! {
    eprintln!("{}", e);
    exit(1);
}

#[derive(clap::Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Quit,
    Subscribe {
        topic: String,
    },
    SendToTopic {
        topic: String,
        qos: u8,
        message: String,
    },
    ExpectOnTopic {
        topic: String,
        qos: u8,
    },
}

#[tokio::main]
async fn main() {
    let args = {
        match std::env::args()
            .skip(1)
            .collect::<String>()
            .split("----")
            .map(|els| Args::try_parse_from(els.split(' ')))
            .collect::<Result<Vec<Args>, _>>()
        {
            Ok(args) => args,
            Err(e) => {
                eprintln!("{}", e);
                exit(1)
            }
        }
    };

    let (client_duplex, server_duplex) = tokio::io::duplex(512);

    let (mut read_dup, mut write_dup) = tokio::io::split(server_duplex);

    tokio::spawn(async move { tokio::io::copy(&mut tokio::io::stdin(), &mut write_dup).await });
    tokio::spawn(async move { tokio::io::copy(&mut read_dup, &mut tokio::io::stdout()).await });

    let client = MqttClient::connect_v3_duplex(
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
    .unwrap_or_else(|e| print_error_and_quit(format!("Could not connect: {e}")));

    tokio::spawn(client.heartbeat(None));

    let packet_stream = client.build_packet_stream().build();
    let mut packet_stream = Box::pin(packet_stream.stream());

    for arg in args {
        match arg.command {
            Some(Command::Quit) => {}
            Some(Command::Subscribe { topic }) => {
                let subscription_requests = [MSubscriptionRequest {
                    topic: MString { value: &topic },
                    qos: MQualityOfService::AtMostOnce, // TODO
                }];
                client.subscribe(&subscription_requests).await.unwrap();
            }
            Some(Command::SendToTopic {
                topic: _,
                qos: _,
                message: _,
            }) => {
                unimplemented!()
            }
            Some(Command::ExpectOnTopic {
                topic: expected_topic,
                qos: expected_qos,
            }) => {
                let packet = match packet_stream.next().await {
                    Some(Ok(packet)) => packet,
                    None => {
                        eprintln!("Stream ended, stopping");
                        break;
                    }
                    Some(Err(error)) => print_error_and_quit(format!("Stream errored: {error}")),
                };

                if let MPacket::Publish(MPublish {
                    qos, topic_name, ..
                }) = packet.get_packet()
                {
                    if topic_name.value != expected_topic {
                        eprintln!(
                            "Expected Publish on topic {}, got on {}",
                            expected_topic, topic_name.value
                        );
                        break;
                    }
                    if qos.to_byte() != expected_qos {
                        eprintln!(
                            "Expected Publish with QoS {}, got {}",
                            expected_qos,
                            qos.to_byte()
                        );
                        break;
                    }
                    // all ok
                } else {
                    eprintln!("Expected Publish, got {:?}", packet.get_packet());
                    break;
                }
            }

            None => {
                // no command, doing nothing
            }
        }
    }
}
