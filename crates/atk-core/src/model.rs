//! Known model identifiers.
//!
//! This table is a name lookup only. It never gates whether a device is
//! queried - that is decided by report geometry in `protocol::detect`. An ATK
//! mouse with an unlisted PID still reports correctly, it just shows up with a
//! generic name. Collected from the Compx / Nordic silicon documentation and
//! from `quiet-battery`'s device support matrix; the `1278` entry is verified
//! on this machine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Receiver,
    Wired,
    Unknown,
}

impl Link {
    pub fn label(self) -> &'static str {
        match self {
            Link::Receiver => "2.4G",
            Link::Wired => "wired",
            Link::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Model {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub link: Link,
}

pub const MODELS: &[Model] = &[
    // ---- Compx 17-byte family -------------------------------------------
    Model {
        name: "ATK F1 Ultimate",
        vid: 0x373B,
        pid: 0x1031,
        link: Link::Receiver,
    },
    Model {
        name: "ATK F1 Ultimate",
        vid: 0x373B,
        pid: 0x102E,
        link: Link::Wired,
    },
    Model {
        name: "ATK A9 Ultimate",
        vid: 0x373B,
        pid: 0x11D9,
        link: Link::Receiver,
    },
    Model {
        name: "ATK A9 Ultimate",
        vid: 0x373B,
        pid: 0x11B6,
        link: Link::Wired,
    },
    Model {
        name: "VXE MAD R",
        vid: 0x373B,
        pid: 0x104D,
        link: Link::Receiver,
    },
    Model {
        name: "VXE MAD R",
        vid: 0x373B,
        pid: 0x103F,
        link: Link::Wired,
    },
    Model {
        name: "VXE MAD R Major Plus",
        vid: 0x373B,
        pid: 0x1040,
        link: Link::Receiver,
    },
    Model {
        name: "VXE MAD R Major Plus",
        vid: 0x373B,
        pid: 0x104C,
        link: Link::Wired,
    },
    Model {
        name: "VXE R1 Pro Max",
        vid: 0x3554,
        pid: 0xF58A,
        link: Link::Receiver,
    },
    Model {
        name: "VXE R1 Pro Max",
        vid: 0x3554,
        pid: 0xF58C,
        link: Link::Wired,
    },
    Model {
        name: "VXE R1 SE+",
        vid: 0x3554,
        pid: 0xF58E,
        link: Link::Receiver,
    },
    Model {
        name: "VXE R1 SE+",
        vid: 0x3554,
        pid: 0xF58F,
        link: Link::Wired,
    },
    Model {
        name: "VGN F1 Pro",
        vid: 0x3554,
        pid: 0xF503,
        link: Link::Receiver,
    },
    Model {
        name: "VGN F1 Pro",
        vid: 0x3554,
        pid: 0xF502,
        link: Link::Wired,
    },
    Model {
        name: "VGN F2 Pro Max",
        vid: 0x3554,
        pid: 0xFB3E,
        link: Link::Receiver,
    },
    Model {
        name: "VGN F2 Pro Max",
        vid: 0x3554,
        pid: 0xFB3D,
        link: Link::Wired,
    },
    // ---- Nordic 54L15, 64-byte family -----------------------------------
    Model {
        name: "ATK Zero (8k dongle-L1)",
        vid: 0x373B,
        pid: 0x1155,
        link: Link::Receiver,
    },
    Model {
        name: "ATK Zero (8k dongle-L0)",
        vid: 0x373B,
        pid: 0x124F,
        link: Link::Receiver,
    },
    Model {
        name: "ATK Zero",
        vid: 0x373B,
        pid: 0x1154,
        link: Link::Wired,
    },
    // ---- Verified on the development machine ----------------------------
    // Confirmed against the official ATK HUB device page on this machine.
    Model {
        name: "ATK X1 Air +",
        vid: 0x373B,
        pid: 0x1278,
        link: Link::Receiver,
    },
];

pub fn lookup(vid: u16, pid: u16) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.vid == vid && m.pid == pid)
}
