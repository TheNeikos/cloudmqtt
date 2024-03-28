//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use clap::Parser;
use cloudmqtt::client::MqttClientConnector;
use cloudmqtt::transport::MqttConnectTransport;
use tokio::net::TcpStream;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    hostname: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let socket = TcpStream::connect(args.hostname).await.unwrap();

    let connection = MqttConnectTransport::TokioTcp(socket);
    let client_id = cloudmqtt::client_identifier::ProposedClientIdentifier::PotentiallyServerProvided;

    let connector = MqttClientConnector::new(
        connection,
        client_id,
        cloudmqtt::client::CleanStart::Yes,
        cloudmqtt::keep_alive::KeepAlive::Disabled,
    );

    let _client = connector.connect().await.unwrap();

    println!("Yay, we connected! That's all for now");
}
