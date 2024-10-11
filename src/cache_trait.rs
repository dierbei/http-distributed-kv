use crate::gossip::{Command, GossipNode, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::{select, time};
use tracing::{info, warn};
const TICK_INTERVAL: Duration = Duration::from_secs(3);

#[async_trait]
/// Trait that defines a basic asynchronous cache (BCache) with common cache operations.
///
/// This trait includes the ability to insert, retrieve, and remove key-value pairs from the cache.
///
/// # Requirements
/// - The implementer of this trait must be thread-safe (`Send` + `Sync`).
/// - All operations are asynchronous, so this trait must be implemented with async functions.
///
/// # Example
///
/// ```rust
/// struct MyCache {
///     // your implementation
/// }
///
/// #[async_trait]
/// impl BCache for MyCache {
///     async fn insert(&mut self, key: String, value: String) {
///         // insert into cache logic
///     }
///
///     async fn get(&mut self, key: String) -> Result<String> {
///         // fetch from cache logic
///         Ok("some_value".to_string())
///     }
///
///     async fn remove(&mut self, key: String) {
///         // remove from cache logic
///     }
/// }
/// ```
///
/// # Errors
///
/// - The `get` function returns a `Result`, so any error during retrieval will be wrapped in an `anyhow::Error`.
pub trait BCache: Send + Sync {
    /// Asynchronously inserts a key-value pair into the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key to be inserted.
    /// * `value` - A `String` representing the value associated with the key.
    async fn insert(&mut self, key: String, value: String);

    /// Asynchronously retrieves the value associated with the given key from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key to be retrieved.
    ///
    /// # Returns
    ///
    /// * A `Result<String>` which contains the value if found, or an error if the key is not found or if any other issue occurs.
    async fn get(&mut self, key: String) -> Result<String>;

    /// Asynchronously removes the key-value pair from the cache, if it exists.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key to be removed.
    async fn remove(&mut self, key: String);
}

/// Asynchronously synchronizes data between an in-memory cache (`bcache`),
/// a gossip network (`gossip`), and an HTTP message receiver.
///
/// This function runs an infinite loop where it periodically performs the following tasks:
///
/// - Sends a `Ping` message to all nodes in the gossip network at a fixed interval.
/// - Listens for incoming gossip messages, deserializes them, and processes them based on their command:
///     - `Ping`: Logs that a ping message was received.
///     - `Insert`: Adds the key-value pair from the gossip message into the cache.
///     - `Remove`: Removes the key from the cache.
/// - Listens for incoming HTTP messages and forwards them to all nodes in the gossip network.
///
/// # Arguments
///
/// * `bcache` - A thread-safe, asynchronous cache implementing the `BCache` trait. Used to store and retrieve key-value pairs.
/// * `gossip` - The gossip network node, responsible for sending and receiving messages across the network.
/// * `gossip_receiver` - A `Receiver` for receiving serialized gossip messages.
/// * `http_receiver` - A `Receiver` for receiving HTTP messages that need to be propagated to the gossip network.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` on success, or an `anyhow::Error` if there is an issue (such as message deserialization failure).
///
/// # Behavior
///
/// - Periodically sends a `Ping` message to all gossip nodes using the `gossip` node.
/// - Processes incoming messages from the gossip network and the HTTP interface, allowing the cache to stay in sync across the system.
///
/// # Errors
///
/// * If a gossip message cannot be deserialized, the function will return an error using `anyhow::Error`.
///
/// # Example
///
/// ```rust
/// sync_data(bcache, gossip, gossip_receiver, http_receiver).await?;
/// ```
///
/// This function will run indefinitely unless interrupted.
///
/// # Panics
///
/// * This function does not handle panics explicitly, but unexpected deserialization failures or locking issues
///   in the cache (`bcache`) might cause runtime errors that would result in early termination.
pub async fn sync_data(
    bcache: Arc<Mutex<Box<dyn BCache>>>,
    gossip: GossipNode,
    mut gossip_receiver: Receiver<Vec<u8>>,
    mut http_receiver: Receiver<Message>,
) -> Result<()> {
    let mut ticker = time::interval(TICK_INTERVAL);

    loop {
        select! {
            _ = ticker.tick() => {
                gossip.send_msg_to_all(Message{key: "".to_string(), value: "".to_string(), cmd: Command::Ping}).await;
            },
            Some(gossip_msg) = gossip_receiver.recv() => {
                match handle_gossip_message(&gossip_msg, &bcache).await {
                    Ok(()) => {},
                    Err(e) => {
                        warn!("Failed to process gossip message: {:?}", e);
                    }
                }
            },
            Some(http_msg) = http_receiver.recv() => {
                println!("receiver http msg: {:?}", http_msg);
                gossip.send_msg_to_all(http_msg).await;
            },
        }
    }
}

async fn handle_gossip_message(
    msg_bytes: &[u8],
    bcache: &Arc<Mutex<Box<dyn BCache>>>,
) -> Result<()> {
    let msg: Message = bincode::deserialize(msg_bytes)
        .map_err(|e| anyhow!("Failed to deserialize message: {:?}", e))?;

    info!("Gossip Message: {:?}", msg);

    match msg.cmd {
        Command::Ping => {
            info!("Received ping message");
        }
        Command::Insert => {
            let mut cache = bcache.lock().await;
            cache.insert(msg.key.clone(), msg.value.clone()).await;
            info!(
                "Message added to cache: {:?}",
                cache.get(msg.key.clone()).await
            );
        }
        Command::Remove => {
            bcache.lock().await.remove(msg.key.clone()).await;
            info!("Message removed from cache");
        }
    }

    Ok(())
}
