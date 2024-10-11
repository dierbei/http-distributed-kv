use anyhow::{anyhow, Result};
use std::net::{SocketAddr, ToSocketAddrs};

/// Parses an optional string into a `SocketAddr`.
///
/// This function attempts to parse an `Option<String>` into a valid `SocketAddr`.
/// If an address is provided, it will attempt to resolve the string into an actual
/// socket address using the `to_socket_addrs` trait. If successful, it returns the
/// resolved `SocketAddr`. Otherwise, it returns an error.
///
/// # Arguments
///
/// * `addr` - An optional `String` representing the address to be parsed. If `None` is provided,
///   an error will be returned.
///
/// # Returns
///
/// * `Ok(SocketAddr)` - If the address is successfully parsed.
/// * `Err(anyhow::Error)` - If no address is provided, or if the provided address string is invalid.
///
/// # Errors
///
/// This function returns an error in the following cases:
///
/// * If no address is provided (`None`).
/// * If the provided address string cannot be parsed into a valid `SocketAddr`.
///
/// # Example
///
/// ```rust
/// let addr = parse_address(Some("127.0.0.1:8080".to_string())).unwrap();
/// assert_eq!(addr, "127.0.0.1:8080".parse().unwrap());
/// ```
///
/// If an invalid address is provided, an error will be returned:
///
/// ```rust
/// let result = parse_address(Some("invalid_address".to_string()));
/// assert!(result.is_err());
/// ```
pub fn parse_address(addr: Option<String>) -> Result<SocketAddr> {
    match addr {
        Some(addr_str) => match addr_str.to_socket_addrs() {
            Ok(mut iter) => {
                if let Some(socket_addr) = iter.next() {
                    Ok(socket_addr)
                } else {
                    Err(anyhow!("Could not parse address"))
                }
            }
            Err(_) => Err(anyhow!("Invalid address format")),
        },
        None => Err(anyhow!("No address provided")),
    }
}
