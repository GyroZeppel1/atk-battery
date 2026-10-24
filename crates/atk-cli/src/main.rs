use atk_core::{scan, DeviceReading, ScanOptions};
use std::time::{Duration, Instant};

const HELP: &str = "\
atk-battery - battery viewer for ATK / VXE / VGN 2.4G mice

USAGE:
    atk-battery [OPTIONS]

OPTIONS:
    --json              machine readable output
    --watch             keep running and re-read on an interval
    --interval <SEC>    refresh period for --watch (default 30)
    --strict            only query PIDs listed in the model table
    --vid <HEX>         restrict to a vendor id, repeatable (default: ATK family)
    --dump              list every HID collection, do not query anything
    --timing            print scan duration to stderr
    --startup [on|off]  create/remove the Startup shortcut (default: status)
    --shortcut [where]  create a launcher shortcut: desktop (default) | startup
    -h, --help          show this help

NOTES:
    Reads are read-only: a single battery query per device, nothing is written
    to the mouse and no vendor driver is started. A sleeping mouse answers
    nothing - nudge it and retry.
";

struct Args {
    json: bool,
    watch: bool,
    interval: u64,
    strict: bool,
    vids: Option<Vec<u16>>,
    dump: bool,
    timing: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        json: false,
        watch: false,
        interval: 30,
        strict: false,
        vids: None,
        dump: false,
        timing: false,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--json" => a.json = true,
            "--watch" => a.watch = true,
            "--strict" => a.strict = true,
            "--dump" => a.dump = true,
            "--timing" => a.timing = true,
            "--interval" => {
                i += 1;
                let v = argv.get(i).ok_or("--interval needs a value")?;
                a.interval = v.parse().map_err(|_| format!("bad interval: {v}"))?;
            }
            "--vid" => {
                i += 1;
                let v = argv.get(i).ok_or("--vid needs a hex value")?;
                let id = u16::from_str_radix(v.trim_start_matches("0x").trim_start_matches("0X"), 16)
                    .map_err(|_| format!("bad vendor id: {v}"))?;
                a.vids.get_or_insert_with(Vec::new).push(id);
            }
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    Ok(a)
}

fn render_table(rows: &[DeviceReading]) -> String {
    if rows.is_empty() {
        return "No ATK mouse found. Is the receiver plugged in?\n".to_string();
    }
    let mut out = String::new();
    out.push_str(&format!(
        "{:<32}  {:<9}  {:<7}  {:>4}  {:>6}  {}\n",
        "DEVICE", "ID", "LINK", "BATT", "CELL", "STATE"
    ));
    out.push_str(&"-".repeat(78));
    out.push('\n');
    for r in rows {
        let id = format!("{:04X}:{:04X}", r.vid, r.pid);
        match &r.result {
            Ok(v) => {
                let mv = match v.millivolts {
                    Some(m) => format!("{:.2} V", m as f64 / 1000.0),
                    None => "-".to_string(),
                };
                out.push_str(&format!(
                    "{:<32}  {:<9}  {:<7}  {:>3}%  {:>6}  {}\n",
                    r.name,
                    id,
                    r.link.label(),
                    v.percent,
                    mv,
                    v.power.label()
                ));
            }
            Err(e) => out.push_str(&format!(
                "{:<32}  {:<9}  {:<7}  {:>4}  {:>6}  {}\n",
                r.name,
                id,
                r.link.label(),
                "--",
                "-",
                e
            )),
        }
    }
    out
}

fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn render_json(rows: &[DeviceReading]) -> String {
    let mut items = Vec::new();
    for r in rows {
        let mut f = vec![
            format!("\"name\":{}", json_escape(&r.name)),
            format!("\"vid\":\"{:04X}\"", r.vid),
            format!("\"pid\":\"{:04X}\"", r.pid),
            format!("\"link\":{}", json_escape(r.link.label())),
            format!("\"protocol\":{}", json_escape(&r.family.to_string())),
            format!("\"known\":{}", r.known),
        ];
        match &r.result {
            Ok(v) => {
                f.push("\"ok\":true".to_string());
                f.push(format!("\"percent\":{}", v.percent));
                f.push(match v.millivolts {
                    Some(m) => format!("\"millivolts\":{m}"),
                    None => "\"millivolts\":null".to_string(),
                });
                f.push(format!("\"power\":{}", json_escape(v.power.label())));
            }
            Err(e) => {
                f.push("\"ok\":false".to_string());
                f.push(format!("\"error\":{}", json_escape(&e.to_string())));
            }
        }
        items.push(format!("  {{{}}}", f.join(",")));
    }
    format!("[\n{}\n]", items.join(",\n"))
}

fn dump_collections() {
    println!(
        "{:<9}  {:<9}  {:>6}  {:>4} {:>4} {:>4}  PATH",
        "VID:PID", "USAGE", "VER", "IN", "OUT", "FEAT"
    );
    for c in atk_core::hid::enumerate() {
        println!(
            "{:04X}:{:04X}  {:04X}:{:04X}  {:>6X}  {:>4} {:>4} {:>4}  {}",
            c.vid, c.pid, c.usage_page, c.usage, c.version, c.input_len, c.output_len,
            c.feature_len, c.path
        );
    }
}

fn run_once(opts: &ScanOptions, args: &Args) -> bool {
    let t0 = Instant::now();
    let rows = scan(opts);
    let elapsed = t0.elapsed();

    if args.json {
        println!("{}", render_json(&rows));
    } else {
        print!("{}", render_table(&rows));
    }
    if args.timing {
        eprintln!("scan took {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    }
    rows.iter().any(|r| r.result.is_ok())
}

fn main() {
    // `--startup` is handled before normal argument parsing: it is a
    // maintenance command, not a scan option.
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(|s| s.as_str()) == Some("--startup") {
        let action = argv.get(1).map(|s| s.as_str()).unwrap_or("status");
        match action {
            "on" | "off" => {
                let enable = action == "on";
                match atk_core::startup::set_enabled(enable) {
                    Ok(p) => {
                        if enable {
                            println!("startup enabled: {}", p.display());
                        } else {
                            println!("startup disabled");
                        }
                        std::process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                let on = atk_core::startup::is_enabled();
                println!("startup: {}", if on { "enabled" } else { "disabled" });
                if let Some(p) = atk_core::startup::startup_lnk_path() {
                    println!("path: {}", p.display());
                }
                std::process::exit(0);
            }
        }
    }

    if argv.first().map(|s| s.as_str()) == Some("--shortcut") {
        let which = argv.get(1).map(|s| s.as_str()).unwrap_or("desktop");
        let (path, extra) = match which {
            "desktop" => (atk_core::startup::desktop_lnk_path(), ""),
            "startup" => (atk_core::startup::startup_lnk_path(), "--tray"),
            other => {
                eprintln!("unknown target: {other} (desktop | startup)");
                std::process::exit(2);
            }
        };
        let Some(path) = path else {
            eprintln!("error: cannot resolve the target folder");
            std::process::exit(1);
        };
        match atk_core::startup::create_shortcut(&path, extra, "ATK 鼠标电量") {
            Ok(_) => {
                println!("shortcut created: {}", path.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }

    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n");
            eprint!("{HELP}");
            std::process::exit(2);
        }
    };

    if args.dump {
        dump_collections();
        return;
    }

    let opts = ScanOptions {
        vids: args.vids.clone().unwrap_or_else(|| atk_core::ATK_VIDS.to_vec()),
        strict: args.strict,
    };

    if !args.watch {
        let any = run_once(&opts, &args);
        std::process::exit(if any { 0 } else { 1 });
    }

    loop {
        // ANSI clear keeps the terminal clean without a curses dependency.
        if !args.json {
            print!("\x1b[2J\x1b[H");
        }
        let any = run_once(&opts, &args);
        if !any && !args.json {
            println!("\nTip: a sleeping mouse answers nothing. Move it and wait for the next poll.");
        }
        std::thread::sleep(Duration::from_secs(args.interval));
    }
}
