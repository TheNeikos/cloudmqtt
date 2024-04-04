//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::collections::HashMap;
use std::collections::VecDeque;
use std::num::NonZeroU16;

use futures::SinkExt;
use mqtt_format::v5::integers::VARIABLE_INTEGER_MAX;
use mqtt_format::v5::packets::publish::MPublish;
use tracing::Instrument;

use super::state::OutstandingPackets;
use super::MqttClient;
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
                .map(|nz| mqtt_format::v5::variable_header::PacketIdentifier(nz.get())),
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
                        .qos1
                        .insert(pi, Qos1Callbacks { on_acknowledge });
                    published_recv = PublishedReceiver::Once(PublishedQos1 { recv });
                }
                QualityOfService::ExactlyOnce => {
                    let (on_receive, recv) = futures::channel::oneshot::channel();
                    let (on_complete, comp_recv) = futures::channel::oneshot::channel();
                    inner.outstanding_callbacks.qos2.insert(
                        pi,
                        Qos2Callbacks {
                            on_receive: Some(on_receive),
                            on_complete: Some(on_complete),
                        },
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
) -> Result<std::num::NonZeroU16, PacketIdentifierExhausted> {
    let start = *next_packet_ident;

    loop {
        let next = *next_packet_ident;

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
    pub(crate) on_packet_recv: Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>,
    pub(crate) handle_acknowledge: Box<dyn Fn(&crate::packets::MqttPacket) -> Acknowledge + Send>,
    // on_receive: Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>,
    // on_complete: Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>,
}

#[derive(Debug)]
pub(crate) enum Acknowledge {
    No,
    Yes,
    YesWithProps {},
}

pub(crate) struct Callbacks {
    pub(crate) ping_req: VecDeque<futures::channel::oneshot::Sender<()>>,
    pub(crate) qos1: HashMap<NonZeroU16, Qos1Callbacks>,
    pub(crate) qos2: HashMap<NonZeroU16, Qos2Callbacks>,
}

impl Callbacks {
    pub(crate) fn new() -> Callbacks {
        Callbacks {
            ping_req: Default::default(),
            qos1: HashMap::default(),
            qos2: HashMap::default(),
        }
    }
}

pub(crate) struct Qos1Callbacks {
    pub(crate) on_acknowledge: futures::channel::oneshot::Sender<crate::packets::MqttPacket>,
}

pub(crate) struct Qos2Callbacks {
    pub(crate) on_receive: Option<futures::channel::oneshot::Sender<crate::packets::MqttPacket>>,
    pub(crate) on_complete: Option<futures::channel::oneshot::Sender<crate::packets::MqttPacket>>,
}

pub struct Publish {
    pub topic: crate::topic::MqttTopic,
    pub qos: QualityOfService,
    pub retain: bool,
    pub payload: MqttPayload,
    pub on_packet_recv: Option<Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>>,
}

pub struct Published {
    recv: PublishedReceiver,
}

impl Published {
    pub async fn acknowledged(self) {
        match self.recv {
            PublishedReceiver::None => return,
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
    recv: futures::channel::oneshot::Receiver<MqttPacket>,
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
    on_packet_recv: Option<Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>>,
}

impl PublishQos1 {
    pub fn with_on_packet_recv(
        mut self,
        on_packet_recv: Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>,
    ) -> Self {
        self.on_packet_recv = Some(on_packet_recv);
        self
    }
}

pub struct PublishQos2 {
    pub topic: crate::topic::MqttTopic,
    pub retain: bool,
    pub payload: MqttPayload,
    on_packet_recv: Option<Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>>,
}

impl PublishQos2 {
    pub fn with_on_packet_recv(
        mut self,
        on_packet_recv: Box<dyn Fn(&crate::packets::MqttPacket) -> () + Send>,
    ) -> Self {
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

        inner.outstanding_callbacks.ping_req.push_back(sender);

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
