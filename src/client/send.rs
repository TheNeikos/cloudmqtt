//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::collections::HashMap;
use std::collections::VecDeque;

use mqtt_format::v5::integers::VARIABLE_INTEGER_MAX;
use mqtt_format::v5::packets::publish::MPublish;
use tracing::Instrument;

use super::state::OutstandingPackets;
use super::MqttClient;
use crate::packet_identifier::PacketIdentifier;
use crate::packets::MqttPacket;
use crate::payload::MqttPayload;
use crate::qos::QualityOfService;

impl MqttClient {
    #[tracing::instrument(skip_all, fields(payload_length = payload.as_ref().len()))]
    pub async fn publish(
        &self,
        Publish {
            topic,
            qos,
            retain,
            payload,
            on_packet_recv: _,
        }: Publish,
    ) -> Result<Published, ()> {
        let mut inner = self.inner.lock().await;
        let inner = &mut *inner;

        let Some(conn_state) = &mut inner.connection_state else {
            tracing::error!("No connection state found");
            return Err(());
        };

        let Some(sess_state) = &mut inner.session_state else {
            tracing::error!("No session state found");
            return Err(());
        };

        if conn_state.retain_available.unwrap_or(true) && retain {
            tracing::warn!("Retain not available, but requested");
            return Err(());
        }

        let packet_identifier = if qos > QualityOfService::AtMostOnce {
            get_next_packet_ident(
                &mut conn_state.next_packet_identifier,
                &sess_state.outstanding_packets,
            )
            .map(Some)
            .map_err(|_| ())? // TODO
        } else {
            None
        };
        tracing::debug!(?packet_identifier, "Packet identifier computed");

        let publish = MPublish {
            duplicate: false,
            quality_of_service: qos.into(),
            retain,
            topic_name: topic.as_ref(),
            packet_identifier: packet_identifier
                .map(mqtt_format::v5::variable_header::PacketIdentifier::from),
            properties: mqtt_format::v5::packets::publish::PublishProperties::new(),
            payload: payload.as_ref(),
        };

        let packet = mqtt_format::v5::packets::MqttPacket::Publish(publish);

        let maximum_packet_size = conn_state
            .maximum_packet_size
            .unwrap_or(VARIABLE_INTEGER_MAX);

        if packet.binary_size() > maximum_packet_size {
            tracing::error!("Binary size bigger than maximum packet size");
            return Err(());
        }

        tracing::trace!(%maximum_packet_size, packet_size = packet.binary_size(), "Packet size");

        let published_recv;

        if let Some(pi) = packet_identifier {
            let mut bytes = tokio_util::bytes::BytesMut::new();
            bytes.reserve(packet.binary_size() as usize);
            let mut writer = crate::packets::MqttWriter(&mut bytes);
            packet.write(&mut writer).map_err(drop)?; // TODO
            let mqtt_packet = crate::packets::MqttPacket {
                packet: yoke::Yoke::try_attach_to_cart(
                    crate::packets::StableBytes(bytes.freeze()),
                    |bytes: &[u8]| mqtt_format::v5::packets::MqttPacket::parse_complete(bytes),
                )
                .unwrap(), // TODO
            };

            sess_state.outstanding_packets.insert(pi, mqtt_packet);
            match qos {
                QualityOfService::AtMostOnce => unreachable!(),
                QualityOfService::AtLeastOnce => {
                    let (on_acknowledge, recv) = futures::channel::oneshot::channel();
                    inner
                        .outstanding_callbacks
                        .add_qos1(pi, Qos1Callbacks { on_acknowledge });
                    published_recv = PublishedReceiver::Once(PublishedQos1 { recv });
                }
                QualityOfService::ExactlyOnce => {
                    let (on_receive, recv) = futures::channel::oneshot::channel();
                    let (on_complete, comp_recv) = futures::channel::oneshot::channel();
                    inner.outstanding_callbacks.add_qos2(
                        pi,
                        Qos2ReceiveCallback { on_receive },
                        Qos2CompleteCallback { on_complete },
                    );
                    published_recv =
                        PublishedReceiver::Twice(PublishedQos2Received { recv, comp_recv });
                }
            }
        } else {
            published_recv = PublishedReceiver::None;
        }

        tracing::trace!("Publishing");
        conn_state
            .conn_write
            .send(packet)
            .in_current_span()
            .await
            .unwrap();
        tracing::trace!("Finished publishing");

        Ok(Published {
            recv: published_recv,
        })
    }

    pub async fn publish_qos1(
        &self,
        PublishQos1 {
            topic,
            retain,
            payload,
            on_packet_recv,
        }: PublishQos1,
    ) -> Result<(), ()> {
        let _res = self
            .publish(Publish {
                topic,
                qos: QualityOfService::AtMostOnce,
                retain,
                payload,
                on_packet_recv,
            })
            .await?;

        Ok(()) // TODO
    }

    pub async fn publish_qos2(
        &self,
        PublishQos2 {
            topic,
            retain,
            payload,
            on_packet_recv,
        }: PublishQos2,
    ) -> Result<(), ()> {
        let _res = self
            .publish(Publish {
                topic,
                qos: QualityOfService::ExactlyOnce,
                retain,
                payload,
                on_packet_recv,
            })
            .await?;

        Ok(()) // TODO
    }
}

fn get_next_packet_ident(
    next_packet_ident: &mut std::num::NonZeroU16,
    outstanding_packets: &OutstandingPackets,
) -> Result<PacketIdentifier, PacketIdentifierExhausted> {
    let start = *next_packet_ident;

    loop {
        let next = PacketIdentifier::from(*next_packet_ident);

        if !outstanding_packets.exists_outstanding_packet(next) {
            return Ok(next);
        }

        match next_packet_ident.checked_add(1) {
            Some(n) => *next_packet_ident = n,
            None => *next_packet_ident = std::num::NonZeroU16::MIN,
        }

        if start == *next_packet_ident {
            return Err(PacketIdentifierExhausted);
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("No free packet identifiers available")]
pub struct PacketIdentifierExhausted;

pub(crate) struct ClientHandlers {
    pub(crate) on_packet_recv: OnPacketRecvFn,
    pub(crate) on_publish_recv: OnPublishRecvFn,
    pub(crate) on_qos1_acknowledge: OnQos1AcknowledgeFn,
    // on_qos2_receive: Box<dyn Fn(crate::packets::MqttPacket) + Send>,
    // on_qos2_complete: Box<dyn Fn(crate::packets::MqttPacket) + Send>,
}

pub type OnPacketRecvFn = Box<dyn Fn(crate::packets::MqttPacket) + Send>;
pub type OnPacketRefRecvFn = Box<dyn Fn(&crate::packets::MqttPacket) + Send>;
pub type OnPublishRecvFn = Box<dyn Fn(crate::packets::Publish) + Send>;
pub type OnQos1AcknowledgeFn = Box<dyn Fn(crate::packets::Puback) + Send>;

impl Default for ClientHandlers {
    fn default() -> Self {
        Self {
            on_packet_recv: Box::new(|_| ()),
            on_publish_recv: Box::new(|_| ()),
            on_qos1_acknowledge: Box::new(|_| ()),
        }
    }
}

#[derive(Debug)]
pub enum Acknowledge {
    No,
    Yes,
    YesWithProps {},
}

pub(crate) struct Callbacks {
    ping_req: VecDeque<futures::channel::oneshot::Sender<()>>,
    qos1: HashMap<PacketIdentifier, Qos1Callbacks>,
    qos2_receive: HashMap<PacketIdentifier, Qos2ReceiveCallback>,
    qos2_complete: HashMap<PacketIdentifier, Qos2CompleteCallback>,
}

impl Callbacks {
    pub(crate) fn new() -> Callbacks {
        Callbacks {
            ping_req: Default::default(),
            qos1: HashMap::default(),
            qos2_receive: HashMap::default(),
            qos2_complete: HashMap::default(),
        }
    }

    pub(crate) fn add_ping_req(&mut self, cb: futures::channel::oneshot::Sender<()>) {
        self.ping_req.push_back(cb);
    }

    pub(crate) fn add_qos1(&mut self, id: PacketIdentifier, cb: Qos1Callbacks) {
        self.qos1.insert(id, cb);
    }

    pub(crate) fn add_qos2(
        &mut self,
        id: PacketIdentifier,
        rec: Qos2ReceiveCallback,
        comp: Qos2CompleteCallback,
    ) {
        self.qos2_receive.insert(id, rec);
        self.qos2_complete.insert(id, comp);
    }

    pub(crate) fn take_ping_req(&mut self) -> Option<futures::channel::oneshot::Sender<()>> {
        self.ping_req.pop_front()
    }

    pub(crate) fn take_qos1(&mut self, id: PacketIdentifier) -> Option<Qos1Callbacks> {
        self.qos1.remove(&id)
    }

    pub(crate) fn take_qos2_receive(
        &mut self,
        id: PacketIdentifier,
    ) -> Option<Qos2ReceiveCallback> {
        self.qos2_receive.remove(&id)
    }

    pub(crate) fn take_qos2_complete(
        &mut self,
        id: PacketIdentifier,
    ) -> Option<Qos2CompleteCallback> {
        self.qos2_complete.remove(&id)
    }
}

pub(crate) struct Qos1Callbacks {
    pub(crate) on_acknowledge: futures::channel::oneshot::Sender<crate::packets::Puback>,
}

pub(crate) struct Qos2ReceiveCallback {
    pub(crate) on_receive: futures::channel::oneshot::Sender<crate::packets::MqttPacket>,
}
pub(crate) struct Qos2CompleteCallback {
    pub(crate) on_complete: futures::channel::oneshot::Sender<crate::packets::MqttPacket>,
}

pub struct Publish {
    pub topic: crate::topic::MqttTopic,
    pub qos: QualityOfService,
    pub retain: bool,
    pub payload: MqttPayload,
    pub on_packet_recv: Option<OnPacketRefRecvFn>,
}

pub struct Published {
    recv: PublishedReceiver,
}

impl Published {
    pub async fn acknowledged(self) {
        match self.recv {
            PublishedReceiver::None => (),
            PublishedReceiver::Once(qos1) => {
                qos1.acknowledged().await;
            }
            PublishedReceiver::Twice(qos2) => {
                qos2.received().await.completed().await;
            }
        }
    }
}

enum PublishedReceiver {
    None,
    Once(PublishedQos1),
    Twice(PublishedQos2Received),
}

pub struct PublishedQos1 {
    recv: futures::channel::oneshot::Receiver<crate::packets::Puback>,
}

impl PublishedQos1 {
    pub async fn acknowledged(self) {
        self.recv.await.unwrap();
    }
}

pub struct PublishedQos2Received {
    recv: futures::channel::oneshot::Receiver<MqttPacket>,
    comp_recv: futures::channel::oneshot::Receiver<MqttPacket>,
}

impl PublishedQos2Received {
    pub async fn received(self) -> PublishedQos2Completed {
        self.recv.await.unwrap();

        PublishedQos2Completed {
            recv: self.comp_recv,
        }
    }
}

pub struct PublishedQos2Completed {
    recv: futures::channel::oneshot::Receiver<MqttPacket>,
}

impl PublishedQos2Completed {
    pub async fn completed(self) {
        self.recv.await.unwrap();
    }
}

pub struct PublishQos1 {
    pub topic: crate::topic::MqttTopic,
    pub retain: bool,
    pub payload: MqttPayload,
    on_packet_recv: Option<OnPacketRefRecvFn>,
}

impl PublishQos1 {
    pub fn with_on_packet_recv(mut self, on_packet_recv: OnPacketRefRecvFn) -> Self {
        self.on_packet_recv = Some(on_packet_recv);
        self
    }
}

pub struct PublishQos2 {
    pub topic: crate::topic::MqttTopic,
    pub retain: bool,
    pub payload: MqttPayload,
    on_packet_recv: Option<OnPacketRefRecvFn>,
}

impl PublishQos2 {
    pub fn with_on_packet_recv(mut self, on_packet_recv: OnPacketRefRecvFn) -> Self {
        self.on_packet_recv = Some(on_packet_recv);
        self
    }
}

impl MqttClient {
    pub async fn ping(&self) -> Result<Ping, ()> {
        let mut inner = self.inner.lock().await;
        let inner = &mut *inner;

        let Some(conn_state) = &mut inner.connection_state else {
            tracing::error!("No connection state found");
            return Err(());
        };

        let packet = mqtt_format::v5::packets::MqttPacket::Pingreq(
            mqtt_format::v5::packets::pingreq::MPingreq,
        );

        let (sender, recv) = futures::channel::oneshot::channel();

        inner.outstanding_callbacks.add_ping_req(sender);

        conn_state.conn_write.send(packet).await.map_err(drop)?;

        Ok(Ping { recv })
    }
}

pub struct Ping {
    recv: futures::channel::oneshot::Receiver<()>,
}

impl Ping {
    pub async fn response(self) {
        self.recv.await.unwrap()
    }
}
