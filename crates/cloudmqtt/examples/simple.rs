//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use cloudmqtt::CloudmqttClient;

#[tokio::main]
async fn main() {
    let mut client = CloudmqttClient::new("localhost:1883".to_string()).await;

    client.wait_for_shutdown().await;
}
