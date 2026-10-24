//! Compx / Nordic 52840 family - the 17-byte report used by most ATK, VXE
//! and VGN 2.4G mice.
//!
//! Request (17 bytes, rest zero):
//!     08 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 49
//!
//! Reply seen on a real ATK receiver at 65%:
//!     08 04 00 00 00 02 41 00 0F 8D 00 00 00 00 00 00 6A
//!                     ^^    ^^^^^
//!                    0x41   0x0F8D
//!                    65%    3981 mV, big endian
//!
//! Byte 5 is a status byte (0x02 = battery valid), byte 7 is the power flag
//! (0 on the 2.4G link, 1 on a direct cable), and byte 16 closes a checksum
//! over all 17 bytes: their sum modulo 256 must equal 0x55.
//!
//! Note the voltage is big endian here and little endian in `nordic54`. Reading
//! it the wrong way round yields a plausible-looking but absurd 36 V.

use crate::{Power, QueryError, Reading};

pub const USAGE_PAGE: u16 = 0xFF02;
pub const USAGE: u16 = 0x0002;
pub const REPORT_ID: u8 = 0x08;
pub const SUB_COMMAND: u8 = 0x04;
pub const CHECKSUM_TARGET: u8 = 0x55;
pub const LEN: usize = 17;

pub fn request() -> [u8; LEN] {
    let mut r = [0u8; LEN];
    r[0] = REPORT_ID;
    r[1] = SUB_COMMAND;
    r[16] = 0x49; // 73 = 0x55 - 0x08 - 0x04
    r
}

pub fn parse(res: &[u8]) -> Result<Reading, QueryError> {
    if res.len() < LEN {
        return Err(QueryError::ShortReply(res.len()));
    }
    if res[0] != REPORT_ID || res[1] != SUB_COMMAND {
        return Err(QueryError::ForeignReply);
    }

    let sum: u32 = res[..LEN].iter().map(|&b| b as u32).sum();
    let got = (sum & 0xFF) as u8;
    if got != CHECKSUM_TARGET {
        return Err(QueryError::BadChecksum { got });
    }

    let percent = res[6];
    if percent > 100 {
        return Err(QueryError::OutOfRange(percent));
    }

    let power = match res[7] {
        0 => Power::Discharging,
        1 if percent >= 100 => Power::Full,
        1 => Power::Charging,
        _ => Power::Unknown,
    };

    let millivolts = ((res[8] as u16) << 8) | res[9] as u16;

    Ok(Reading {
        percent,
        millivolts: Some(millivolts),
        power,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real 373B:1278 receiver.
    const CAPTURE: [u8; 17] = [
        0x08, 0x04, 0x00, 0x00, 0x00, 0x02, 0x41, 0x00, 0x0F, 0x8D, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x6A,
    ];

    #[test]
    fn request_checksum_is_valid() {
        let r = request();
        assert_eq!(r[0], 0x08);
        assert_eq!(r[1], 0x04);
        assert_eq!(r.iter().map(|&b| b as u32).sum::<u32>() & 0xFF, 0x55);
    }

    #[test]
    fn parses_real_capture() {
        let r = parse(&CAPTURE).unwrap();
        assert_eq!(r.percent, 65);
        assert_eq!(r.millivolts, Some(3981));
        assert_eq!(r.power, Power::Discharging);
    }

    #[test]
    fn rejects_corrupted_checksum() {
        let mut bad = CAPTURE;
        bad[16] = 0x00;
        assert!(matches!(parse(&bad), Err(QueryError::BadChecksum { .. })));
    }

    #[test]
    fn rejects_echo_of_our_own_request() {
        let mut echo = request();
        echo[16] = 0x00; // break checksum so it cannot pass by luck
        assert!(parse(&echo).is_err());
    }

    #[test]
    fn rejects_out_of_range_percent() {
        let mut bad = CAPTURE;
        bad[6] = 0xFF;
        // repair checksum so only the range check can reject it
        let sum: u32 = bad[..16].iter().map(|&b| b as u32).sum();
        bad[16] = (0x55u32.wrapping_sub(sum) & 0xFF) as u8;
        assert!(matches!(parse(&bad), Err(QueryError::OutOfRange(255))));
    }
}
