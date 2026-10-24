//! Panel painting. Everything is drawn with GDI primitives - no controls, no
//! layout engine, so the visual result is identical on any Windows version.

use crate::icon::battery_color;
use crate::winui::*;
use atk_core::win::{wide, HANDLE};
use atk_core::{DeviceReading, Power, QueryError};

const BG: u32 = rgb(255, 255, 255);
const INK: u32 = rgb(31, 31, 31);
const MUTED: u32 = rgb(95, 94, 90);
const HINT: u32 = rgb(138, 138, 138);
const TRACK: u32 = rgb(232, 232, 232);

unsafe fn text(hdc: HANDLE, font: HANDLE, s: &str, color: u32, rc: &mut RECT, fmt: u32) {
    let old = SelectObject(hdc, font);
    SetTextColor(hdc, color);
    SetBkMode(hdc, TRANSPARENT);
    let w = wide(s);
    DrawTextW(hdc, w.as_ptr(), -1, rc, fmt);
    SelectObject(hdc, old);
}

pub unsafe fn paint(
    hdc: HANDLE,
    rows: &[DeviceReading],
    fonts: &Fonts,
    updated: &str,
    w: i32,
    h: i32,
) {
    let full = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    let brush = CreateSolidBrush(BG);
    FillRect(hdc, &full, brush);
    DeleteObject(brush);

    let dev = rows
        .iter()
        .find(|r| r.result.is_ok())
        .or_else(|| rows.first());
    let Some(dev) = dev else {
        let mut rc = RECT {
            left: 0,
            top: 60,
            right: w,
            bottom: 100,
        };
        text(
            hdc,
            fonts.mid,
            "未检测到 ATK 设备",
            INK,
            &mut rc,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        let mut rc2 = RECT {
            left: 0,
            top: 100,
            right: w,
            bottom: 130,
        };
        text(
            hdc,
            fonts.small,
            "请确认接收器已插入，或移动鼠标唤醒",
            MUTED,
            &mut rc2,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        return;
    };

    // device name
    let mut rc = RECT {
        left: 26,
        top: 18,
        right: w - 26,
        bottom: 44,
    };
    text(
        hdc,
        fonts.mid,
        &dev.name,
        MUTED,
        &mut rc,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );

    match &dev.result {
        Err(e) => {
            let mut rc = RECT {
                left: 26,
                top: 52,
                right: w - 26,
                bottom: 100,
            };
            text(
                hdc,
                fonts.big,
                "--",
                INK,
                &mut rc,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let mut rc2 = RECT {
                left: 26,
                top: 104,
                right: w - 26,
                bottom: 130,
            };
            let msg = match e {
                QueryError::Timeout | QueryError::NoReply => "鼠标未响应",
                QueryError::Asleep => "鼠标休眠中",
                other => return_msg(other),
            };
            text(
                hdc,
                fonts.small,
                msg,
                MUTED,
                &mut rc2,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let mut rc3 = RECT {
                left: 26,
                top: 138,
                right: w - 26,
                bottom: 162,
            };
            text(
                hdc,
                fonts.small,
                "动一下鼠标，几秒后会自动刷新",
                HINT,
                &mut rc3,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            return;
        }
        Ok(v) => {
            let pct = v.percent;
            let charging = matches!(v.power, Power::Charging | Power::Full);
            let accent = battery_color(pct, charging);

            // big number, then the % sign sized to sit beside it
            let num = pct.to_string();
            let mut measure = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            {
                let old = SelectObject(hdc, fonts.big);
                let wnum = wide(&num);
                DrawTextW(
                    hdc,
                    wnum.as_ptr(),
                    -1,
                    &mut measure,
                    DT_CALCRECT | DT_SINGLELINE,
                );
                SelectObject(hdc, old);
            }
            let num_w = measure.right - measure.left;

            let mut rc_num = RECT {
                left: 26,
                top: 46,
                right: w - 26,
                bottom: 112,
            };
            text(
                hdc,
                fonts.big,
                &num,
                INK,
                &mut rc_num,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );

            let mut rc_pct = RECT {
                left: 26 + num_w + 3,
                top: 66,
                right: w - 26,
                bottom: 112,
            };
            text(
                hdc,
                fonts.mid,
                "%",
                accent,
                &mut rc_pct,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );

            // right side: cell voltage + state
            if let Some(mv) = v.millivolts {
                let mut rc_v = RECT {
                    left: 26,
                    top: 58,
                    right: w - 26,
                    bottom: 82,
                };
                text(
                    hdc,
                    fonts.mid,
                    &format!("{:.2} V", mv as f64 / 1000.0),
                    MUTED,
                    &mut rc_v,
                    DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
                );
            }
            let state = match v.power {
                Power::Discharging => "放电中",
                Power::Charging => "充电中",
                Power::Full => "已充满",
                Power::Unknown => "",
            };
            if !state.is_empty() {
                let mut rc_s = RECT {
                    left: 26,
                    top: 80,
                    right: w - 26,
                    bottom: 102,
                };
                text(
                    hdc,
                    fonts.small,
                    state,
                    HINT,
                    &mut rc_s,
                    DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
                );
            }

            // progress track + fill
            let track_h = 10;
            let ty = 124;
            // A brush must be selected out of the DC before DeleteObject,
            // otherwise the delete fails and the handle leaks on every repaint.
            let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));

            let track_brush = CreateSolidBrush(TRACK);
            let old_brush = SelectObject(hdc, track_brush);
            RoundRect(hdc, 26, ty, w - 26, ty + track_h, track_h, track_h);
            SelectObject(hdc, old_brush);
            DeleteObject(track_brush);

            let fill_w = (((w - 52) as f32) * (pct as f32 / 100.0)).round() as i32;
            if fill_w > 2 {
                let fill_brush = CreateSolidBrush(accent);
                let fb = SelectObject(hdc, fill_brush);
                RoundRect(
                    hdc,
                    26,
                    ty,
                    26 + fill_w.max(track_h),
                    ty + track_h,
                    track_h,
                    track_h,
                );
                SelectObject(hdc, fb);
                DeleteObject(fill_brush);
            }
            SelectObject(hdc, old_pen);

            // footer
            let footer = if updated.is_empty() {
                format!("{} · {}", dev.link.label(), dev.family)
            } else {
                format!("更新于 {updated} · {}", dev.link.label())
            };
            let mut rc_f = RECT {
                left: 26,
                top: ty + 18,
                right: w - 26,
                bottom: ty + 40,
            };
            text(
                hdc,
                fonts.small,
                &footer,
                HINT,
                &mut rc_f,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    if rows.len() > 1 {
        let mut rc_m = RECT {
            left: 26,
            top: h - 26,
            right: w - 26,
            bottom: h - 8,
        };
        text(
            hdc,
            fonts.small,
            &format!("共 {} 台设备", rows.len()),
            HINT,
            &mut rc_m,
            DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
        );
    }
}

fn return_msg(e: &QueryError) -> &'static str {
    match e {
        QueryError::Io(_) => "读取失败",
        QueryError::ShortReply(_) => "回复数据不完整",
        QueryError::ForeignReply => "回复与请求不匹配",
        QueryError::BadChecksum { .. } => "数据校验失败",
        QueryError::OutOfRange(_) => "电量值异常",
        QueryError::Timeout | QueryError::NoReply => "鼠标未响应",
        QueryError::Asleep => "鼠标休眠中",
    }
}
