//
// MIT License
//
// Copyright (c) 2018-2019 Stegos AG
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{Network, NetworkProvider, UnicastMessage};
use failure::Error;
use futures::prelude::*;
use futures::sync::mpsc;
use log::*;
use stegos_crypto::pbc::secure;

#[derive(Debug, Clone)]
pub struct DummyNetwork {
    control_tx: mpsc::UnboundedSender<ControlMessage>,
}

impl DummyNetwork {
    pub fn new() -> (Network, impl Future<Item = (), Error = ()>) {
        let (service, control_tx) = DummyNetworkService::new();
        (Box::new(DummyNetwork { control_tx }), service)
    }
}

impl NetworkProvider for DummyNetwork {
    /// Subscribe to topic, returns Stream<Vec<u8>> of messages incoming to topic
    fn subscribe(&self, topic: &str) -> Result<mpsc::UnboundedReceiver<Vec<u8>>, Error> {
        let topic: String = topic.clone().into();
        let (tx, rx) = mpsc::unbounded();
        self.control_tx
            .unbounded_send(ControlMessage::Subscribe { topic, handler: tx })?;
        Ok(rx)
    }

    /// Published message to topic
    fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), Error> {
        let topic: String = topic.clone().into();
        self.control_tx
            .unbounded_send(ControlMessage::Publish { topic, data })?;
        Ok(())
    }

    // Subscribe to unicast messages
    fn subscribe_unicast(
        &self,
        _protocol_id: &str,
    ) -> Result<mpsc::UnboundedReceiver<UnicastMessage>, Error> {
        let (tx, rx) = mpsc::unbounded::<UnicastMessage>();
        let msg = ControlMessage::SubscribeUnicast { consumer: tx };
        self.control_tx.unbounded_send(msg)?;
        Ok(rx)
    }

    // Send direct message to public key
    fn send(&self, to: secure::PublicKey, _protocol_id: &str, data: Vec<u8>) -> Result<(), Error> {
        let msg = ControlMessage::SendUnicast { to, data };
        self.control_tx.unbounded_send(msg)?;
        Ok(())
    }

    // Clone self as a box
    fn box_clone(&self) -> Network {
        Box::new((*self).clone())
    }
}

struct DummyNetworkService {}

impl DummyNetworkService {
    fn new() -> (
        impl Future<Item = (), Error = ()>,
        mpsc::UnboundedSender<ControlMessage>,
    ) {
        let mut consumers: Vec<mpsc::UnboundedSender<Vec<u8>>> = Vec::new();
        let mut unicast_consumers: Vec<mpsc::UnboundedSender<UnicastMessage>> = Vec::new();
        let (tx, mut rx) = mpsc::unbounded();

        let fut = futures::future::poll_fn(move || -> Result<_, ()> {
            loop {
                match rx.poll() {
                    Ok(Async::Ready(Some(msg))) => match msg {
                        ControlMessage::Publish { topic: _, data: _ } => (),
                        ControlMessage::Subscribe { topic: _, handler } => consumers.push(handler),
                        ControlMessage::SubscribeUnicast { consumer } => {
                            unicast_consumers.push(consumer)
                        }
                        ControlMessage::SendUnicast { to: _, data: _ } => (),
                    },
                    Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                    Ok(Async::NotReady) => break,
                    Err(_) => error!("Error in DummyNetwork Future!"),
                }
            }
            Ok(Async::NotReady)
        });
        (fut, tx)
    }
}

#[derive(Clone, Debug)]
pub enum ControlMessage {
    Subscribe {
        topic: String,
        handler: mpsc::UnboundedSender<Vec<u8>>,
    },
    Publish {
        topic: String,
        data: Vec<u8>,
    },
    SendUnicast {
        to: secure::PublicKey,
        data: Vec<u8>,
    },
    SubscribeUnicast {
        consumer: mpsc::UnboundedSender<UnicastMessage>,
    },
}
