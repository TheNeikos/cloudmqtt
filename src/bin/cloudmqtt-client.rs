use clap::Parser;
use cloudmqtt::{packet_stream::Acknowledge, MqttClient, MqttConnectionParams, MqttPacket};
use futures::StreamExt;
use mqtt_format::v3::{
    packet::MPacket, strings::MString, subscription_request::MSubscriptionRequest, will::MLastWill,
};
use tracing::error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(long, value_parser)]
    addr: String,
    #[clap(long, value_parser)]
    client_id: String,

    #[clap(long, value_parser)]
    subscriptions: Vec<String>,
}

#[tokio::main]
async fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_timer(tracing_subscriber::fmt::time::uptime());

    let filter_layer = tracing_subscriber::EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    tracing::info!("Starting up");

    let args = Args::parse();

    let mut client = MqttClient::connect_v3_unsecured(
        &args.addr,
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
                value: &args.client_id,
            },
        },
    )
    .await
    .unwrap();

    tokio::spawn(client.hearbeat(None));

    client
        .subscribe(
            &args
                .subscriptions
                .iter()
                .map(|sub| MSubscriptionRequest {
                    topic: MString { value: &sub },
                    qos: mqtt_format::v3::qos::MQualityOfService::AtLeastOnce,
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();

    let packet_stream = client
        .build_packet_stream()
        .with_custom_ack_fn(|packet| async move {
            println!("ACKing packet {packet:?}");
            Acknowledge
        })
        .build();
    let mut packet_stream = Box::pin(packet_stream.stream());

    loop {
        let packet = match packet_stream.next().await {
            Some(Ok(packet)) => packet,
            None => {
                error!("Stream closed unexpectedly");
                break;
            }
            Some(Err(error)) => {
                error!(?error, "Stream errored");
                break;
            }
        };

        let packet = packet.get_packet().unwrap();
        println!("Received: {packet:#?}");
    }
}
