//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use cloudmqtt::CloudmqttClient;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let mut client = CloudmqttClient::new("localhost:1883".to_string()).await;

    client.publish(b"What's up", "foo/bar").await;

    client.subscribe("whats/up").await;

    {
        let receive_messages = client.receive_messages();

        let mut receive_messages = std::pin::pin!(receive_messages);

        while let Some(msg) = receive_messages.next().await {
            println!("Got: {msg:?}");
        }
    }

    client.wait_for_shutdown().await;
}
