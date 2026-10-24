//! Protocol families and their reply parsers.
//!
//! Detection is capability based, not model based: a collection qualifies by
//! usage page plus exact report geometry. Model tables only supply a display
//! name - an unlisted PID with the right shape still works, which is how the
//! `373B:1278` unit on this machine was identified.

pub mod compx52;
pub mod nordic54;

use crate::hid::Collection;
use crate::Family;

pub fn detect(c: &Collection) -> Option<Family> {
    if c.usage_page == compx52::USAGE_PAGE
        && c.usage == compx52::USAGE
        && c.input_len == 17
        && c.output_len == 17
    {
        return Some(Family::Compx52);
    }
    if c.usage_page == nordic54::USAGE_PAGE && c.input_len == 64 && c.output_len == 64 {
        return Some(Family::Nordic54);
    }
    None
}
