use std::{
    fmt::Display,
    time::{Duration, SystemTime, SystemTimeError},
};

use either::Either;
use itertools::Itertools;
use wireguard_control::{PeerConfig, PeerStats};
use yansi::Paint;

use crate::models::{AugmentedDevice, AugmentedPeerInfo};

macro_rules! cond_writeln {
    ($($x: expr),* ; $o: expr) => {
        if let Some(o) = $o{
            writeln!($($x),* ,o )
        } else { Ok(())}
    };
}

impl Display for AugmentedDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            name,
            public_key,
            listen_port,
            peers,
            fwmark,
            ..
        } = self;
        writeln!(f, "{}: {}", "interface".green().bold(), name.green())?;
        cond_writeln!(f, "  {}: {}", "public key".bold(); public_key.as_ref().map(|key| key.to_base64()))?;
        #[allow(clippy::write_literal)]
        //should look for WG_HIDE_KEYS == never -> display
        writeln!(f, "  {}: {}", "private key".bold(), "(hidden)")?;
        cond_writeln!(f, "  {}: {}", "listening port".bold(); listen_port)?;
        cond_writeln!(f, "  {}: {:#x}", "fwmark".bold(); fwmark.filter(|m| *m != 0))?;
        for peer in peers {
            write!(f, "\n{}", peer)?;
        }
        Ok(())
    }
}

fn format_bytes(f: &mut std::fmt::Formatter<'_>, b: u64) -> std::fmt::Result {
    let (b, s): (Either<u64, f64>, _) = if b < 1024 {
        (Either::Left(b), "B")
    } else if b < 1024 * 1024u64 {
        (Either::Right(b as f64 / 1024.0), "KiB")
    } else if b < 1024 * 1024 * 1024u64 {
        (Either::Right(b as f64 / (1024 * 1024u64) as f64), "MiB")
    } else if b < 1024 * 1024 * 1024 * 1024u64 {
        (
            Either::Right(b as f64 / (1024 * 1024 * 1024u64) as f64),
            "GiB",
        )
    } else {
        (
            Either::Right(b as f64 / (1024 * 1024 * 1024 * 1024u64) as f64),
            "TiB",
        )
    };
    match b {
        Either::Left(b) => write!(f, "{} {}", b, s.cyan()),
        Either::Right(b) => write!(f, "{:.2} {}", b, s.cyan()),
    }
}

fn format_duration(f: &mut std::fmt::Formatter<'_>, d: Duration) -> std::fmt::Result {
    let secs_total = d.as_secs();
    let secs = secs_total % 60;
    let mins = (secs_total / 60) % 60;
    let hours = (secs_total / 3600) % 24;
    let days = secs_total / (3600 * 24) % 356;
    let years = secs_total / (356 * 3600 * 24);

    macro_rules! fmt_time {
        ($plural: literal, $singular: literal, $count: expr , $last: expr) => {
            if $count > 0 {
                let comma = if $last { "" } else { "," };
                if $count == 1 {
                    write!(f, "{} {}{} ", $count, $singular.cyan(), comma)
                } else {
                    write!(f, "{} {}{} ", $count, $plural.cyan(), comma)
                }
            } else {
                Ok(())
            }
        };
    }

    fmt_time!("years", "year", years, false)?;
    fmt_time!("days", "day", days, false)?;
    fmt_time!("hours", "hour", hours, false)?;
    fmt_time!("minutes", "minute", mins, false)?;
    fmt_time!("seconds", "second", secs, false)?;
    Ok(())
}

pub(crate) fn format_last_handshake_symbol(
    time: Option<Result<Duration, SystemTimeError>>,
) -> String {
    match time.and_then(|r| r.map(|o| o.as_secs()).ok()) {
        Some(s) => {
            if s < 150 {
                "🟢"
            } else {
                "🟡"
            }
        }
        None => "🔵",
    }
    .to_string()
}

impl Display for AugmentedPeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            config:
                PeerConfig {
                    public_key,
                    preshared_key,
                    endpoint,
                    persistent_keepalive_interval,
                    allowed_ips,
                    ..
                },
            stats:
                PeerStats {
                    last_handshake_time,
                    rx_bytes,
                    tx_bytes,
                },
            name,
        } = self;
        let time_since_last_handshake =
            last_handshake_time.map(|t| SystemTime::now().duration_since(t));
        let symbol = format_last_handshake_symbol(time_since_last_handshake.clone());
        match name {
            Some(name) => writeln!(
                f,
                "{}: {} {} {}",
                "peer".yellow().bold(),
                symbol,
                name.yellow(),
                ("(".to_string() + &public_key.to_base64() + ")").yellow()
            )?,
            None => writeln!(
                f,
                "{}: {}",
                "peer".yellow().bold(),
                public_key.to_base64().yellow()
            )?,
        }

        cond_writeln!(f, "  {}: {}", "friendly name".bold(); name)?;
        cond_writeln!(f, "  {}: {}", "preshared key".bold(); preshared_key.as_ref().map(|e| e.to_base64()).filter(|e| e != "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="))?;
        cond_writeln!(f, "  {}: {}", "endpoint ".bold(); endpoint)?;
        let allowed_ips: String = if !allowed_ips.is_empty() {
            Itertools::intersperse(
                allowed_ips.iter().map(|i| format!("{:?}", i)),
                ", ".to_string(),
            )
            .collect()
        } else {
            "(none)".to_string()
        };
        writeln!(f, "  {}: {}", "allowed ips".bold(), allowed_ips)?;
        match time_since_last_handshake {
            Some(Err(_)) => {
                write!(
                    f,
                    "({})",
                    "System clock wound backward; connection problems may ensue.".red()
                )?;
            }
            Some(Ok(o)) => {
                write!(f, "  {}: ", "latest handshake".bold())?;
                format_duration(f, o)?;
                writeln!(f, "ago")?;
            }
            None => {}
        };
        // let time_since = Some(1u16);

        write!(f, "  {}:", "transfer ".bold())?;
        format_bytes(f, *rx_bytes)?;
        write!(f, " received, ")?;
        format_bytes(f, *tx_bytes)?;
        writeln!(f, " sent")?;

        cond_writeln!(f, "  {0}: every {2} {1}", "persistent keepalive".bold(), "seconds".cyan(); persistent_keepalive_interval)?;
        // writeln!(f, "  {} {}", "endpoint:".bold())?;

        Ok(())
    }
}
