use anyhow::{anyhow, Context, Result};
use std::cmp::PartialEq;
use std::error::Error;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::utils::parse_address;
use async_trait::async_trait;
use gossipod::{
    config::{GossipodConfigBuilder, NetworkType},
    DispatchEventHandler, Gossipod, Node, NodeMetadata,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self};
use tokio::time;
use tracing::{error, info};

pub struct GossipNode {
    pub gossipod: Arc<Gossipod>,
    config: gossipod::config::GossipodConfig,
}

pub struct GossipodConfig {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub join_addr: Option<String>,
}

impl GossipodConfig {
    pub fn new(name: String, addr: String, join_addr: Option<String>) -> Self {
        let gossip_addr = parse_address(Some(addr)).unwrap();
        let ip = gossip_addr.ip().to_string();
        let port = gossip_addr.port();

        Self {
            name,
            ip,
            port,
            join_addr,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Ping,
    Insert,
    Remove,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub cmd: Command,
    pub key: String,
    pub value: String,
}

struct EventHandler {
    sender: mpsc::Sender<Vec<u8>>,
}

impl EventHandler {
    fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        Self { sender }
    }
}

type DispatchError = Box<dyn Error + Send + Sync>;

#[async_trait]
impl<M: NodeMetadata> DispatchEventHandler<M> for EventHandler {
    async fn notify_dead(&self, node: &Node<M>) -> Result<(), DispatchError> {
        info!("Node {} detected as dead", node.name);
        Ok(())
    }

    async fn notify_leave(&self, node: &Node<M>) -> Result<(), DispatchError> {
        info!("Node {} is leaving the cluster", node.name);
        Ok(())
    }

    async fn notify_join(&self, node: &Node<M>) -> Result<(), DispatchError> {
        info!("Node {} has joined the cluster", node.name);
        Ok(())
    }

    async fn notify_message(
        &self,
        from: SocketAddr,
        message: Vec<u8>,
    ) -> Result<(), DispatchError> {
        info!("Received message from {}: {:?}", from, message);
        self.sender.send(message).await?;
        Ok(())
    }
}

impl GossipNode {
    pub async fn start(args: GossipodConfig) -> Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let config = GossipodConfigBuilder::new()
            .with_name(&args.name)
            .with_port(args.port)
            .with_addr(args.ip.parse::<Ipv4Addr>().expect("Invalid IP address"))
            .with_probing_interval(Duration::from_secs(5))
            .with_ack_timeout(Duration::from_millis(500))
            .with_indirect_ack_timeout(Duration::from_secs(1))
            .with_suspicious_timeout(Duration::from_secs(5))
            .with_network_type(NetworkType::Local)
            .build()
            .await?;

        let (sender, receiver) = mpsc::channel(1000);
        let dispatch_event_handler = EventHandler::new(sender);

        let gossipod =
            Gossipod::with_event_handler(config.clone(), Arc::new(dispatch_event_handler))
                .await
                .context("Failed to initialize Gossipod with custom metadata")?;

        let mut gossip = GossipNode {
            gossipod: gossipod.into(),
            config,
        };
        gossip.start_node().await?;
        gossip.join_node(args.join_addr.clone()).await?;

        Ok((gossip, receiver))
    }

    async fn join_node(&mut self, join_addr: Option<String>) -> Result<()> {
        if let Some(join_addr) = join_addr {
            return match join_addr.parse::<SocketAddr>() {
                Ok(addr) => {
                    if let Err(e) = self.gossipod.join(addr).await {
                        return Err(anyhow!("Failed to join {}: {:?}", addr, e));
                    } else {
                        info!("Successfully joined {}", addr);
                        Ok(())
                    }
                }
                Err(e) => Err(anyhow!("Invalid join address {}: {:?}", join_addr, e)),
            };
        }

        info!("No join address specified. Running as a standalone node.");

        Ok(())
    }

    async fn start_node(&self) -> Result<()> {
        let gossipod_clone = self.gossipod.clone();
        tokio::spawn(async move {
            if let Err(e) = gossipod_clone.start().await {
                error!("[ERR] Error starting Gossipod: {:?}", e);
            }
        });

        while !self.gossipod.is_running().await {
            time::sleep(Duration::from_millis(100)).await;
        }

        let local_node = self.gossipod.get_local_node().await?;
        info!("Local node: {}:{}", local_node.ip_addr, local_node.port);

        Ok(())
    }

    pub async fn send_msg_to_all(&self, msg: Message) {
        for node in self.gossipod.members().await.unwrap_or_default() {
            if node.name == self.config.name() {
                continue; // skip self
            }
            let target = node.socket_addr().unwrap();
            info!(
                "Sending to {}: key={} value={} target={}",
                node.name, msg.key, msg.value, target
            );
            if let Err(e) = self
                .gossipod
                .send(target, &bincode::serialize(&msg).unwrap())
                .await
            {
                error!("Failed to send message to {}: {}", node.name, e);
            }
        }
    }
}
