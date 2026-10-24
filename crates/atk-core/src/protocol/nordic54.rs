//! Nordic 54L15 family - the 64-byte report used by the ATK Zero.
//!
//! Request (64 bytes, rest zero):
//!     08 7D 72 02 00 01 07 01 00 ...
//!
//! Reply:
//!     08 72 00 3A 00 07 01 64 16 11 00 ...
//!
//! Byte 2 is a status: 0 means the mouse itself answered, 1 means it did not
//! (asleep, out of range, or the wrong link index). Bytes 5-6 echo the request
//! so a stale reply from a previous transaction can be rejected. Byte 7 is the
//! percentage; bytes 8-9 are the cell voltage in millivolts, little endian
//! *here* - the 17-byte family above uses big endian for the same quantity.
//!
//! Byte 5 of the request is the link index: 0 reaches a mouse on the cable,
//! 1 reaches one behind the dongle. Picking the wrong one gives either silence
//! or a replay flagged status=1.

use crate::{Power, QueryError, Reading};

pub const USAGE_PAGE: u16 = 0xFF05;
pub const REPORT_ID: u8 = 0x08;
pub const CMD: u8 = 0x72;
pub const SUB_BATTERY: u8 = 0x07;
pub const LEN: usize = 64;

pub fn request(link: u8) -> [u8; LEN] {
    let mut r = [0u8; LEN];
    r[0] = REPORT_ID;
    r[1] = 0x7D;
    r[2] = CMD;
    r[3] = 0x02;
    r[5] = link;
    r[6] = SUB_BATTERY;
    r[7] = 0x01;
    r
}

/// `wired` selects how the power flag is interpreted; the wire also makes the
/// voltage meaningful, over the air those bytes are junk.
pub fn parse(res: &[u8], wired: bool) -> Result<Reading, QueryError> {
    if res.len() < 10 {
        return Err(QueryError::ShortReply(res.len()));
    }
    if res[1] != CMD || res[5] != SUB_BATTERY {
        return Err(QueryError::ForeignReply);
    }
    if res[2] != 0x00 {
        return Err(QueryError::Asleep);
    }

    let percent = res[7];
    if percent > 100 {
        return Err(QueryError::OutOfRange(percent));
    }

    let power = if !wired {
        Power::Discharging
    } else if percent >= 100 {
        Power::Full
    } else {
        Power::Charging
    };

    let millivolts = if wired {
        Some((res[8] as u16) | ((res[9] as u16) << 8))
    } else {
        None
    };

    Ok(Reading {
        percent,
        millivolts,
        power,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(status: u8, pct: u8, mv: u16) -> [u8; LEN] {
        let mut r = [0u8; LEN];
        r[0] = 0x08;
        r[1] = CMD;
        r[2] = status;
        r[3] = 0x3A;
        r[5] = SUB_BATTERY;
        r[6] = 0x01;
        r[7] = pct;
        r[8] = (mv & 0xFF) as u8;
        r[9] = (mv >> 8) as u8;
        r
    }

    #[test]
    fn request_matches_capture() {
        let r = request(1);
        assert_eq!(&r[..8], &[0x08, 0x7D, 0x72, 0x02, 0x00, 0x01, 0x07, 0x01]);
        assert_eq!(r[8..], [0u8; 56]);
        assert_eq!(request(0)[5], 0x00);
    }

    #[test]
    fn parses_wired_full() {
        let r = parse(&reply(0, 100, 4374), true).unwrap();
        assert_eq!(r.percent, 100);
        assert_eq!(r.millivolts, Some(4374));
        assert_eq!(r.power, Power::Full);
    }

    #[test]
    fn wireless_has_no_voltage_and_discharges() {
        let r = parse(&reply(0, 64, 0xDEAD), false).unwrap();
        assert_eq!(r.percent, 64);
        assert_eq!(r.millivolts, None);
        assert_eq!(r.power, Power::Discharging);
    }

    #[test]
    fn status_one_is_asleep_not_a_battery_value() {
        assert!(matches!(parse(&reply(1, 100, 0), true), Err(QueryError::Asleep)));
    }

    #[test]
    fn rejects_reply_with_wrong_echo() {
        let mut bad = reply(0, 50, 3900);
        bad[5] = 0x08;
        assert!(matches!(parse(&bad, true), Err(QueryError::ForeignReply)));
    }

    #[test]
    fn rejects_out_of_range_percent() {
        assert!(matches!(
            parse(&reply(0, 0xFF, 3900), true),
            Err(QueryError::OutOfRange(255))
        ));
    }
}
