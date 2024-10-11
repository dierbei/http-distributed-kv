use clap::Parser;
use std::sync::Arc;
mod cache_trait;
mod foyer_cache;
mod gossip;
mod http_server;
mod log;
mod utils;

use crate::cache_trait::{sync_data, BCache};
use crate::foyer_cache::FoyerCache;
use crate::gossip::{GossipNode, GossipodConfig};
use anyhow::Result;
use tokio::sync::Mutex;
use tracing::info;

/// Command-line arguments for the application.
///
/// This struct defines the necessary arguments for starting the application,
/// such as the node's name, HTTP server address, Gossip protocol address,
/// cache capacity, and an optional Gossip join address.
///
/// # Fields
///
/// - `name`: The name of the Gossip node, passed using `-n` or `--name`.
/// - `http_addr`: The address for the HTTP server, passed using `--http-addr`.
///   Defaults to `0.0.0.0:3001`.
/// - `gossip_addr`: The address for the Gossip protocol, passed using `-g` or `--gossip-addr`.
///   Defaults to `0.0.0.0:4001`.
/// - `cache_capacity`: The maximum capacity for the in-memory cache, passed using `-c` or `--cache-capacity`.
///   Defaults to `128`.
/// - `gossip_join_addr`: An optional address for joining an existing Gossip network, passed using `--gossip-join-addr`.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    name: String,

    #[arg(long, default_value = "0.0.0.0:3001")]
    http_addr: String,

    #[arg(short, long, default_value = "0.0.0.0:4001")]
    gossip_addr: String,

    #[arg(short, long, default_value_t = 128)]
    cache_capacity: usize,

    #[arg(long)]
    gossip_join_addr: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initializing the log and parsing parameters
    log::setup_tracing();
    let args = Args::parse();
    info!("Starting application with arguments: {:?}", args);

    // Starting a GossipNode
    let (gossip, gossip_receiver) = GossipNode::start(GossipodConfig::new(
        args.name,
        args.gossip_addr,
        args.gossip_join_addr,
    ))
    .await?;

    // Creating a Cache
    let bcache: Arc<Mutex<Box<dyn BCache>>> = Arc::new(Mutex::new(Box::new(
        FoyerCache::new(args.cache_capacity).await,
    )));

    // Starting the HTTP server
    let http_receiver = http_server::start(args.http_addr.clone(), bcache.clone()).await?;
    info!("HTTP server started on {}", args.http_addr);

    // Synchronize Gossip and HTTP data
    sync_data(bcache, gossip, gossip_receiver, http_receiver).await?;

    Ok(())
}
