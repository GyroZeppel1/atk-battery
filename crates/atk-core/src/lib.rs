//! Battery reads for ATK-family 2.4G mice.
//!
//! Layering mirrors the reference projects but inverts one decision: discovery
//! and protocol selection are driven by the HID report geometry a device
//! actually exposes, not by a hard-coded model list. The model table is only
//! used for display names.

pub mod hid;
mod hidapi;
pub mod model;
pub mod protocol;
pub mod startup;
pub mod win;

use std::fmt;

/// Vendor ids seen on ATK / VXE / VGN / rebadged Compx receivers.
pub const ATK_VIDS: &[u16] = &[0x373B, 0x3554, 0x260D, 0x33E4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// 17-byte report, usage page 0xFF02, checksum modulo 256 = 0x55.
    Compx52,
    /// 64-byte report, usage page 0xFF05, link index selects cable or dongle.
    Nordic54,
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Family::Compx52 => f.write_str("compx-17"),
            Family::Nordic54 => f.write_str("nordic-64"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Discharging,
    Charging,
    Full,
    Unknown,
}

impl Power {
    pub fn label(self) -> &'static str {
        match self {
            Power::Discharging => "discharging",
            Power::Charging => "charging",
            Power::Full => "full",
            Power::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reading {
    pub percent: u8,
    /// The 17-byte family reports cell voltage; the 64-byte family only over
    /// the cable. Treat as advisory - the gauge is voltage based and rounds.
    pub millivolts: Option<u16>,
    pub power: Power,
}

#[derive(Debug, Clone)]
pub enum QueryError {
    Io(String),
    Timeout,
    NoReply,
    Asleep,
    ShortReply(usize),
    ForeignReply,
    BadChecksum { got: u8 },
    OutOfRange(u8),
}

impl QueryError {
    fn from_io(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::TimedOut {
            QueryError::Timeout
        } else {
            QueryError::Io(e.to_string())
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::Io(s) => write!(f, "I/O error: {s}"),
            QueryError::Timeout => f.write_str("no reply (timed out)"),
            QueryError::NoReply => f.write_str("no reply"),
            QueryError::Asleep => f.write_str("mouse asleep or out of range"),
            QueryError::ShortReply(n) => write!(f, "short reply ({n} bytes)"),
            QueryError::ForeignReply => f.write_str("reply did not match the request"),
            QueryError::BadChecksum { got } => {
                write!(f, "checksum mismatch (got 0x{got:02X}, expected 0x55)")
            }
            QueryError::OutOfRange(v) => write!(f, "battery value {v} out of range"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceReading {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub link: model::Link,
    pub family: Family,
    pub path: String,
    pub known: bool,
    pub result: Result<Reading, QueryError>,
}

pub struct ScanOptions {
    pub vids: Vec<u16>,
    /// Only touch devices that appear in the model table.
    pub strict: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            vids: ATK_VIDS.to_vec(),
            strict: false,
        }
    }
}

pub fn scan(opts: &ScanOptions) -> Vec<DeviceReading> {
    let mut out: Vec<DeviceReading> = Vec::new();

    for c in hid::enumerate() {
        if !opts.vids.contains(&c.vid) {
            continue;
        }
        let Some(family) = protocol::detect(&c) else {
            continue;
        };
        let known = model::lookup(c.vid, c.pid);
        if opts.strict && known.is_none() {
            continue;
        }
        if out.iter().any(|d| d.vid == c.vid && d.pid == c.pid) {
            continue;
        }

        let (name, link) = match known {
            Some(m) => (m.name.to_string(), m.link),
            None => (format!("ATK family {:04X}:{:04X}", c.vid, c.pid), model::Link::Unknown),
        };

        let result = query(&c, family);
        out.push(DeviceReading {
            name,
            vid: c.vid,
            pid: c.pid,
            link,
            family,
            path: c.path,
            known: known.is_some(),
            result,
        });
    }

    out
}

fn query(col: &hid::Collection, family: Family) -> Result<Reading, QueryError> {
    let ep = hid::Endpoint::open(&col.path, true).map_err(|e| QueryError::from_io(e))?;
    let in_len = col.input_len as usize;

    match family {
        Family::Compx52 => {
            ep.drain(in_len, 3);
            ep.write(&protocol::compx52::request(), 250)
                .map_err(QueryError::from_io)?;
            let res = ep.read(in_len, 250).map_err(QueryError::from_io)?;
            protocol::compx52::parse(&res)
        }
        Family::Nordic54 => {
            // Link index 1 reaches a mouse behind the dongle, 0 reaches one on
            // the cable. The wrong index is either silent or replays a stale
            // answer flagged status=1, so try the dongle first and fall back.
            let mut last = QueryError::NoReply;
            for link in [1u8, 0u8] {
                ep.drain(in_len, 3);
                if ep.write(&protocol::nordic54::request(link), 250).is_err() {
                    continue;
                }
                match ep.read(in_len, 250) {
                    Ok(res) => match protocol::nordic54::parse(&res, link == 0) {
                        Ok(r) => return Ok(r),
                        Err(e) => last = e,
                    },
                    Err(e) => last = QueryError::from_io(e),
                }
            }
            Err(last)
        }
    }
}
