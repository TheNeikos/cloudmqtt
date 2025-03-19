//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

mod codec;

use std::time::Instant;

use cloudmqtt_core::client::ExpectedAction;
use cloudmqtt_core::client::MqttClientFSM;
use cloudmqtt_core::client::MqttInstant;
use codec::MqttPacketCodec;
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;

fn since(start: Instant) -> MqttInstant {
    MqttInstant::new(start.elapsed().as_secs())
}

pub struct CloudmqttClient {}

impl CloudmqttClient {
    pub async fn new(address: String) -> CloudmqttClient {
        let socket = tokio::net::lookup_host(address)
            .await
            .expect("Could not lookup DNS")
            .next()
            .expect("DNS resolved to no addresses");

        let mut connection = tokio::net::TcpStream::connect(socket)
            .await
            .expect("Could not connect");

        let mut fsm = MqttClientFSM::new();

        let start = Instant::now();

        let (reader, mut writer) = connection.split();

        let mut writer = FramedWrite::new(&mut writer, MqttPacketCodec);
        let mut reader = FramedRead::new(reader, MqttPacketCodec);

        let action = fsm.handle_connect(
            since(start),
            mqtt_format::v5::packets::connect::MConnect {
                client_identifier: "cloudmqtt-0",
                username: None,
                password: None,
                clean_start: true,
                will: None,
                properties: mqtt_format::v5::packets::connect::ConnectProperties::new(),
                keep_alive: 0,
            },
        );

        match action {
            ExpectedAction::SendPacket(mqtt_packet) => {
                writer
                    .send(mqtt_packet)
                    .await
                    .expect("Could not send message");
            }
            _ => unreachable!(),
        }

        loop {
            let packet = reader
                .next()
                .await
                .expect("The next message to be a connack")
                .expect("Parsing to succeed");

            {
                let fsm: &mut MqttClientFSM = &mut fsm;
                if let Some(action) = fsm.consume(packet.get_packet()).run(since(start)) {
                    match action {
                        ExpectedAction::SendPacket(mqtt_packet) => {
                            writer
                                .send(mqtt_packet)
                                .await
                                .expect("Could not send packet");
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }

        CloudmqttClient {}
    }
}
