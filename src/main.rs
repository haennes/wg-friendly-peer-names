#![feature(anonymous_lifetime_in_impl_trait)]
use std::{
    borrow::Borrow,
    collections::HashMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use clap_complete::{generate, CompletionCandidate};
use debug_print::debug_println;
use itertools::Itertools;
use serde::Serialize;
use wireguard_control::{Backend, Device, InterfaceName, PeerConfig, PeerStats};
use yansi::{Condition, Paint};

use clap::{CommandFactory, Parser};

use crate::{
    format::format_last_handshake_symbol,
    models::{
        Args, AugmentedDevice, AugmentedPeerInfo, OutputFormat, PeerFileFormatError,
        WgFriendlyPeerNameError,
    },
};

pub mod format;
pub(crate) mod models;

fn main() -> Result<(), WgFriendlyPeerNameError> {
    let Args {
        peerfile_path,
        names_only,
        peername,
        ifname,
        force_color,
        show_completions,
        output_format,
    } = Args::parse();

    if let Some(shell) = show_completions {
        generate(shell, &mut Args::command(), "wgg", &mut io::stdout());
        return Ok(());
    }

    if !force_color {
        yansi::whenever(Condition::TTY_AND_COLOR);
    }
    let ifs = match ifname {
        None => Device::list(Backend::Kernel).map_err(WgFriendlyPeerNameError::ListingError)?,
        Some(name) => vec![InterfaceName::from_str(&name)
            .map_err(WgFriendlyPeerNameError::InvalidInterfaceError)?],
    };
    let ifs = ifs.iter();
    let if_names = load_if_names(peerfile_path)?;
    let mut augemented = augment_devices(get_devices(ifs)?, &if_names);
    #[allow(clippy::manual_inspect)]
    //False positive: See https://github.com/rust-lang/rust-clippy/issues/13185
    let devs: Vec<_> = match peername {
        Some(peername) => augemented
            .iter_mut()
            .map(|dev: &mut AugmentedDevice| {
                dev.peers
                    .retain(|peer| peer.name.as_ref().is_some_and(|n| *n == peername));
                dev
            })
            .collect(),
        None => augemented.iter_mut().collect(),
    };
    match output_format {
        OutputFormat::Text => {
            if names_only {
                output_names(devs)
            } else {
                output_all(devs)
            }
        }
        OutputFormat::Json => {
            if names_only {
                #[derive(Serialize)]
                struct OuterOutputJson {
                    if_name: String,
                    peers: Vec<String>,
                }
                let l = devs
                    .iter()
                    .map(|d| OuterOutputJson {
                        if_name: d.name.to_string(),
                        peers: d
                            .peers
                            .iter()
                            .map(|p| p.name.clone().unwrap_or(p.config.public_key.to_base64()))
                            .collect_vec(),
                    })
                    .collect_vec();
                println!(
                    "{}",
                    serde_json::to_string(&l).unwrap_or_else(|e| e.to_string())
                );
                Ok(())
            } else {
                let fmt = serde_json::to_string(&devs).unwrap_or_else(|e| e.to_string());
                println!("{}", fmt);
                Ok(())
            }
        }
    }
}

fn load_if_names(
    peerfile_path: PathBuf,
) -> Result<HashMap<String, String>, WgFriendlyPeerNameError> {
    let peer_file_contents =
        fs::read_to_string(peerfile_path).map_err(WgFriendlyPeerNameError::PeerFileOpenError)?;
    let mut if_names = HashMap::<String, String>::new();
    for (idx, line) in peer_file_contents.lines().enumerate() {
        let (key, name) =
            line.split_once(':')
                .ok_or(WgFriendlyPeerNameError::PeerFileFormatError(
                    PeerFileFormatError::MissingColon(idx + 1),
                ))?;
        if_names.insert(key.to_string(), name.to_string());
    }
    Ok(if_names)
}

fn augment_device(dev: Device, if_names: &HashMap<String, String>) -> AugmentedDevice {
    let Device {
        name,
        public_key,
        private_key,
        fwmark,
        listen_port,
        linked_name,
        backend,
        peers,
        ..
    } = dev;
    let peers = peers
        .into_iter()
        .map(|p| AugmentedPeerInfo::new(if_names.get(&p.config.public_key.to_base64()).cloned(), p))
        .collect();
    AugmentedDevice {
        name,
        public_key,
        private_key,
        fwmark,
        listen_port,
        peers,
        linked_name,
        backend,
    }
}
fn augment_devices(
    ifs_infos: Vec<Device>,
    if_names: &HashMap<String, String>,
) -> Vec<AugmentedDevice> {
    ifs_infos
        .into_iter()
        .map(|dev| augment_device(dev, if_names))
        .collect()
}

fn get_devices(
    ifs: impl Iterator<Item = &InterfaceName>,
) -> Result<Vec<Device>, WgFriendlyPeerNameError> {
    ifs.map(|int| Device::get(int, Backend::Kernel).map_err(|e| (int, e)))
        .process_results(|e| e.collect::<Vec<_>>())
        .map_err(|(int, e)| WgFriendlyPeerNameError::InterfaceQueryError(*int, e))
}

fn output_names<IT, T>(devs: IT) -> Result<(), WgFriendlyPeerNameError>
where
    IT: IntoIterator<Item = T>,
    T: Borrow<AugmentedDevice>,
{
    for dev in devs {
        let AugmentedDevice { name, peers, .. } = dev.borrow();
        println!("{}: {}", "interface".green().bold(), name.green());
        for AugmentedPeerInfo {
            config: PeerConfig { public_key, .. },
            stats: PeerStats {
                last_handshake_time,
                ..
            },
            name: peer_name,
        } in peers
        {
            println!(
                "{}: {} {}",
                "peer".yellow().bold(),
                peer_name.as_ref().unwrap_or(&public_key.to_base64()),
                format_last_handshake_symbol(
                    last_handshake_time.map(|t| SystemTime::now().duration_since(t))
                )
            )
        }
    }
    Ok(())
}

fn output_all<IT, T>(devs: IT) -> Result<(), WgFriendlyPeerNameError>
where
    IT: IntoIterator<Item = T>,
    T: Borrow<AugmentedDevice>,
{
    for device in devs {
        println!("{}", device.borrow())
    }

    Ok(())
}

fn complete_peer_name(current: &OsStr) -> Vec<CompletionCandidate> {
    fn _complete_peer_name(
        current: &OsStr,
    ) -> Result<Vec<CompletionCandidate>, WgFriendlyPeerNameError> {
        // waiting for https://github.com/clap-rs/clap/discussions/5708
        // just assume the keyfile is at /etc/wireguard/peers
        let ifs = Device::list(Backend::Kernel).map_err(WgFriendlyPeerNameError::ListingError)?;
        let if_names = load_if_names(Path::new("/etc/wireguard/peers").into())?;
        let augemented = augment_devices(get_devices(ifs.iter())?, &if_names);
        let peers = augemented
            .iter()
            .flat_map(|d| {
                d.peers.iter().filter(|p| {
                    p.name.as_ref().is_some_and(|name| {
                        if_names.contains_key(name)
                            && name.starts_with(&current.to_string_lossy().to_string())
                    })
                })
            })
            .filter_map(|peer| peer.name.clone());
        let peers = peers.map(CompletionCandidate::new).collect();
        Ok(peers)
    }
    _complete_peer_name(current).unwrap_or_default()
}

fn complete_if_name(current: &OsStr) -> Vec<CompletionCandidate> {
    fn _complete_if_name(
        current: &OsStr,
    ) -> Result<Vec<CompletionCandidate>, WgFriendlyPeerNameError> {
        let if_active_names = Device::list(Backend::Kernel)
            .map_err(WgFriendlyPeerNameError::ListingError)?
            .into_iter()
            .map(|i| i.as_str_lossy().to_string());
        let if_file_names = load_if_names(Path::new("/etc/wireguard/peers").into())?;
        Ok(if_active_names
            .filter(|n| {
                if_file_names.contains_key(n)
                    && n.starts_with(&current.to_string_lossy().to_string())
            })
            .map(CompletionCandidate::new)
            .collect())
    }
    _complete_if_name(current).unwrap_or_default()
}
