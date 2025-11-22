use clap_complete::{ArgValueCompleter, Shell};
use thiserror::Error;
use wireguard_control::{
    Backend, Device, InterfaceName, InvalidInterfaceName, Key, PeerConfig, PeerInfo, PeerStats,
};

use clap::{Parser, ValueHint};

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
        add = ArgValueCompleter::new(complete_peer_name)
    )]
    pub(crate) peername: Option<String>,

    #[arg(
        long = "ifname",
        short = 'i',
        help = "Show only information abut this interface",
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
        hide = true,
        long = "completions",
        help = "show completions and immediately exit",
        value_enum
    )]
    pub(crate) show_completions: Option<Shell>,
}

//TODO acutally use some kind of inheritance
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AugmentedPeerInfo {
    pub config: PeerConfig,
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
//TODO acutally use some kind of inheritance
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AugmentedDevice {
    /// The interface name of this device
    pub name: InterfaceName,
    /// The public encryption key of this interface (if present)
    pub public_key: Option<Key>,
    /// The private encryption key of this interface (if present)
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
    pub backend: Backend,
}
