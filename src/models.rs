use std::{
    net::{IpAddr, SocketAddr},
    time::SystemTime,
};

use clap_complete::{ArgValueCompleter, Shell};
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    *,
};
use thiserror::Error;
use wireguard_control::{
    AllowedIp, Backend, Device, InterfaceName, InvalidInterfaceName, Key, PeerConfig, PeerInfo,
    PeerStats,
};

use clap::{Parser, ValueEnum, ValueHint};

use crate::{complete_if_name, complete_peer_name};

#[derive(Error, Debug)]
pub enum PeerFileFormatError {
    #[error("missing colon on line {0}")]
    MissingColon(usize),
}

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)] //Error postfix is fine
pub enum WgFriendlyPeerNameError {
    #[error("error while listing devices {0}")]
    ListingError(std::io::Error),

    #[error("invalid interface name {0}")]
    InvalidInterfaceError(InvalidInterfaceName),

    #[error("error while querying information for interface {0}")]
    InterfaceQueryError(InterfaceName, std::io::Error),

    #[error("error opening peer file {0}")]
    PeerFileOpenError(std::io::Error),

    #[error("error in peer file format \n {0}")]
    PeerFileFormatError(PeerFileFormatError),
}

#[derive(Parser)]
#[command(name = "wgg")]
pub(crate) struct Args {
    #[arg(
        long = "peerfile",
        help = "The peerfile to read, defaults to /etc/wireguard/peers",
        // default_value = OsString::from("/etc/wireguard/peers")
        default_value = "/etc/wireguard/peers",
        value_hint = ValueHint::FilePath
    )]
    pub(crate) peerfile_path: std::path::PathBuf,

    #[arg(
        long = "onlynames",
        short = 'n',
        help = "If active only show the names, not the other connection parameters",
        action = clap::ArgAction::SetTrue
    )]
    pub(crate) names_only: bool,

    #[arg(
        long = "peername",
        short = 'p',
        help = "Show only information about this peer",
        value_name = "PEER",
        add = ArgValueCompleter::new(complete_peer_name)
    )]
    pub(crate) peername: Option<String>,

    #[arg(
        long = "ifname",
        short = 'i',
        help = "Show only information abut this interface",
        value_name = "IF",
        add = ArgValueCompleter::new(complete_if_name)
    )]
    pub(crate) ifname: Option<String>,

    #[arg(
        long = "force_color",
        short = 'c',
        help = "Wether to always display colors",
        action = clap::ArgAction::SetTrue

    )]
    pub(crate) force_color: bool,

    #[arg(
        long = "outputformat",
        short = 'f',
        help = "What format to Output",
        default_value_t = OutputFormat::Text,
        value_enum
    )]
    pub(crate) output_format: OutputFormat,

    #[arg(
        hide = true,
        long = "completions",
        help = "show completions and immediately exit",
        value_enum
    )]
    pub(crate) show_completions: Option<Shell>,
}

#[derive(Eq, PartialEq, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

fn allowed_ips_serializer<S: Serializer>(v: &Vec<AllowedIp>, s: S) -> Result<S::Ok, S::Error> {
    let mut s_seq = s.serialize_seq(Some(v.len()))?;
    for e in v {
        #[derive(Serialize)]
        struct AllowedIp {
            /// The IP address.
            pub address: IpAddr,
            /// The CIDR subnet mask.
            pub cidr: u8,
        }

        let wireguard_control::AllowedIp { address, cidr } = e;
        s_seq.serialize_element(&AllowedIp {
            address: address.clone(),
            cidr: *cidr,
        })?;
    }
    s_seq.end()
}

#[derive(Serialize)]
#[non_exhaustive]
#[serde(remote = "PeerConfig")]
pub struct PeerConfigSer {
    /// The public key of the peer.
    #[serde(serialize_with = "key_serializer")]
    pub public_key: Key,
    /// The preshared key available to both peers (`None` means no PSK is used).
    #[serde(serialize_with = "key_serializer_opt")]
    pub preshared_key: Option<Key>,
    /// The endpoint this peer listens for connections on (`None` means any).
    pub endpoint: Option<SocketAddr>,
    /// The interval for sending keepalive packets (`None` means disabled).
    pub persistent_keepalive_interval: Option<u16>,
    /// The IP addresses this peer is allowed to have.
    #[serde(serialize_with = "allowed_ips_serializer")]
    pub allowed_ips: Vec<AllowedIp>,
}

#[derive(Serialize)]
#[serde(remote = "PeerStats")]
pub struct PeerStatsSer {
    /// Time of the last handshake/rekey with this peer.
    pub last_handshake_time: Option<SystemTime>,
    /// Number of bytes received from this peer.
    pub rx_bytes: u64,
    /// Number of bytes transmitted to this peer.
    pub tx_bytes: u64,
}

//TODO acutally use some kind of inheritance
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct AugmentedPeerInfo {
    #[serde(with = "PeerConfigSer")]
    pub config: PeerConfig,
    #[serde(with = "PeerStatsSer")]
    pub stats: PeerStats,
    pub name: Option<String>,
}

impl AugmentedPeerInfo {
    pub(crate) fn new(name: Option<String>, PeerInfo { config, stats }: PeerInfo) -> Self {
        Self {
            config,
            stats,
            name,
        }
    }
}
fn if_name_serializer<S: Serializer>(v: &InterfaceName, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

fn key_serializer_opt<S: Serializer>(v: &Option<Key>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => s.serialize_str(&v.to_base64()),
        None => s.serialize_none(),
    }
}
fn key_serializer<S: Serializer>(v: &Key, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_base64())
}

#[derive(Serialize)]
#[serde(remote = "Backend")]
pub enum BackendSer {
    #[cfg(target_os = "linux")]
    Kernel,
    #[cfg(target_os = "openbsd")]
    OpenBSD,
    Userspace,
}

//TODO acutally use some kind of inheritance
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct AugmentedDevice {
    /// The interface name of this device
    #[serde(serialize_with = "if_name_serializer")]
    pub name: InterfaceName,
    /// The public encryption key of this interface (if present)
    #[serde(serialize_with = "key_serializer_opt")]
    pub public_key: Option<Key>,
    /// The private encryption key of this interface (if present)
    #[serde(serialize_with = "key_serializer_opt")]
    pub private_key: Option<Key>,
    /// The [fwmark](https://www.linux.org/docs/man8/tc-fw.html) of this interface
    pub fwmark: Option<u32>,
    /// The port to listen for incoming connections on
    pub listen_port: Option<u16>,
    /// The list of all registered peers and their information
    pub peers: Vec<AugmentedPeerInfo>,
    /// The associated "real name" of the interface (ex. "utun8" on macOS).
    pub linked_name: Option<String>,
    /// The backend the device exists on (userspace or kernel).
    #[serde(with = "BackendSer")]
    pub backend: Backend,
}
