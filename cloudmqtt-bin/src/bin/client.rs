//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::time::Duration;

use clap::Parser;
use cloudmqtt::client::connect::MqttClientConnector;
use cloudmqtt::client::send::Publish;
use cloudmqtt::client::MqttClient;
use cloudmqtt::transport::MqttConnectTransport;
use futures::FutureExt;
use tokio::net::TcpStream;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    hostname: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let filter = tracing_subscriber::filter::EnvFilter::from_default_env();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .pretty();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    let socket = TcpStream::connect(args.hostname).await.unwrap();

    let connection = MqttConnectTransport::TokioTcp(socket);
    let client_id =
        cloudmqtt::client_identifier::ProposedClientIdentifier::PotentiallyServerProvided;

    let connector = MqttClientConnector::new(
        connection,
        client_id,
        cloudmqtt::client::connect::CleanStart::Yes,
        cloudmqtt::keep_alive::KeepAlive::Seconds(5.try_into().unwrap()),
    );

    let client = MqttClient::builder()
        .with_on_packet_recv(Box::new(|packet| {
            tracing::trace!(?packet, "Received packet")
        }))
        .with_handle_acknowledge(Box::new(|packet| {
            async move {
                tracing::trace!(?packet, "Acknowledging packet");
                cloudmqtt::client::send::Acknowledge::Yes
            }
            .boxed()
        }))
        .build()
        .await
        .unwrap();
    let connected = client.connect(connector).await.unwrap();
    let background = tokio::task::spawn(connected.background_task);

    client
        .publish(Publish {
            topic: "foo/bar".try_into().unwrap(),
            qos: cloudmqtt::qos::QualityOfService::ExactlyOnce,
            retain: false,
            payload: vec![123].try_into().unwrap(),
            on_packet_recv: None,
        })
        .await
        .unwrap()
        .acknowledged()
        .await;

    client.ping().await.unwrap().response().await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    client
        .publish(Publish {
            topic: "foo/bar".try_into().unwrap(),
            qos: cloudmqtt::qos::QualityOfService::AtMostOnce,
            retain: false,
            payload: vec![123].try_into().unwrap(),
            on_packet_recv: None,
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(20)).await;

    println!("Sent message! Bye");
}
