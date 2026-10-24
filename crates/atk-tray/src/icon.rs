//! Runtime icon generation: a ring gauge that stays legible at 16 px.
//!
//! An earlier version drew the percentage as a glyph, but two digits in a
//! 32x32 tile scale down to 16 px and turn to mush in the tray. A ring reads
//! at a glance without parsing text: the filled arc *is* the battery level,
//! and colour carries the warning state.

use crate::winui::*;
use atk_core::win::HANDLE;
use std::ffi::c_void;
use std::ptr;

const SIZE: i32 = 32;

const RING_MID: f32 = 11.5; // radius through the middle of the stroke
const RING_HALF: f32 = 2.1; // half the stroke width
const FEATHER: f32 = 0.9; // soft outer edge, keeps it from looking jagged

const TRACK: u32 = rgb(214, 214, 214);

pub fn battery_color(percent: u8, charging: bool) -> u32 {
    if charging {
        return rgb(55, 138, 221);
    }
    if percent >= 60 {
        rgb(29, 158, 117)
    } else if percent >= 30 {
        rgb(186, 117, 23)
    } else {
        rgb(216, 90, 48)
    }
}

pub fn muted_color() -> u32 {
    rgb(136, 135, 128)
}

fn split(c: u32) -> (u8, u8, u8) {
    (
        (c & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        ((c >> 16) & 0xFF) as u8,
    )
}

/// `idle` draws a full grey ring plus a centre dot - "connected but unknown"
/// rather than "0%", which would look like a flat battery.
fn build(percent: u8, color: u32, idle: bool) -> HANDLE {
    unsafe {
        let screen = GetDC(ptr::null_mut());
        if screen.is_null() {
            return ptr::null_mut();
        }
        let dc = CreateCompatibleDC(screen);
        if dc.is_null() {
            ReleaseDC(ptr::null_mut(), screen);
            return ptr::null_mut();
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = SIZE;
        bmi.bmiHeader.biHeight = SIZE; // bottom-up, as icon bitmaps expect
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = ptr::null_mut();
        let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
        if bmp.is_null() || bits.is_null() {
            DeleteDC(dc);
            ReleaseDC(ptr::null_mut(), screen);
            return ptr::null_mut();
        }

        let (cr, cg, cb) = split(color);
        let (tr, tg, tb) = split(TRACK);
        let filled_deg = percent as f32 * 3.6;

        let px = bits as *mut u8;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let dx = x as f32 + 0.5 - 16.0;
                let dy = y as f32 + 0.5 - 16.0;
                let d = (dx * dx + dy * dy).sqrt();

                let mut rgba = [0u8; 4];

                let band = (d - RING_MID).abs();
                if band <= RING_HALF + FEATHER {
                    // 0 deg at 12 o'clock, increasing clockwise
                    let mut ang = dy.atan2(dx).to_degrees() + 90.0;
                    if ang < 0.0 {
                        ang += 360.0;
                    }
                    let on = idle || ang <= filled_deg;
                    let (r, g, b) = if on { (cr, cg, cb) } else { (tr, tg, tb) };
                    let a = if band <= RING_HALF {
                        255.0
                    } else {
                        ((RING_HALF + FEATHER - band) / FEATHER * 255.0).clamp(0.0, 255.0)
                    };
                    rgba = [r, g, b, a as u8];
                } else if idle && d < 3.0 {
                    let a = if d < 2.2 {
                        255
                    } else {
                        (((3.0 - d) / 0.8) * 255.0).clamp(0.0, 255.0) as u8
                    };
                    rgba = [tr, tg, tb, a];
                }

                // bottom-up rows: image row 0 is the last row in memory
                let py = SIZE - 1 - y;
                let off = ((py * SIZE + x) * 4) as usize;
                let p = px.add(off);
                *p = rgba[2];
                *p.add(1) = rgba[1];
                *p.add(2) = rgba[0];
                *p.add(3) = rgba[3];
            }
        }

        // all-zero AND mask: alpha decides visibility
        let mask = CreateBitmap(SIZE, SIZE, 1, 1, ptr::null());
        let mdc = CreateCompatibleDC(screen);
        let old_mask = SelectObject(mdc, mask);
        let full = RECT {
            left: 0,
            top: 0,
            right: SIZE,
            bottom: SIZE,
        };
        let black = CreateSolidBrush(rgb(0, 0, 0));
        FillRect(mdc, &full, black);
        DeleteObject(black);
        SelectObject(mdc, old_mask);
        DeleteDC(mdc);

        let info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: bmp,
        };
        let icon = CreateIconIndirect(&info);

        DeleteObject(mask);
        DeleteObject(bmp);
        DeleteDC(dc);
        ReleaseDC(ptr::null_mut(), screen);

        icon
    }
}

pub fn make_icon(percent: u8, charging: bool) -> HANDLE {
    build(percent, battery_color(percent, charging), false)
}

/// Neutral ring for "connected but no reading": not 0%, not an error.
pub fn make_idle_icon() -> HANDLE {
    build(0, muted_color(), true)
}
