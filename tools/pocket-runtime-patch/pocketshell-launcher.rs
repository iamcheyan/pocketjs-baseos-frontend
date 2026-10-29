use std::process::{Command, Stdio};

/// Run an external command with a hard timeout. This runs on the guest's single
/// render thread, so it must never be able to block the UI: `pactl` on this
/// device blocks forever waiting for a PulseAudio socket, which froze the whole
/// launcher (black screen, dead input) until it was wrapped in `timeout`.
fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut argv: Vec<&str> = vec!["2", program];
    argv.extend_from_slice(args);
    match Command::new("/usr/bin/timeout")
        .args(&argv)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => Some(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => None,
    }
}
const LANGUAGE_PATH: &str = "/storage/pocketshell/launcher/settings.json";

fn language_command(fields: &[&str]) -> String {
    match fields {
        ["LANGUAGE"] => std::fs::read_to_string(LANGUAGE_PATH)
            .ok()
            .and_then(|value| {
                let language = value.trim().trim_matches('"');
                ["ja", "zh", "en"].contains(&language).then(|| language.to_owned())
            })
            .unwrap_or_else(|| "ja".into()),
        ["LANGUAGE", language] if ["ja", "zh", "en"].contains(language) => {
            let path = std::path::Path::new(LANGUAGE_PATH);
            let Some(parent) = path.parent() else { return "Unable to save language".into() };
            if std::fs::create_dir_all(parent).is_err() { return "Unable to save language".into() }
            let temporary = parent.join("settings.json.new");
            if std::fs::write(&temporary, format!("\"{language}\"\n")).is_err()
                || std::fs::rename(&temporary, path).is_err() {
                let _ = std::fs::remove_file(temporary);
                return "Unable to save language".into();
            }
            (*language).into()
        }
        _ => "Invalid language".into(),
    }
}


fn read(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|value| value.trim_matches('\0').trim().to_owned())
}

fn os_release(key: &str) -> Option<String> {
    read("/etc/os-release")?.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then(|| value.trim_matches('"').to_owned())
    })
}

fn cpu_frequency_mhz() -> Option<u64> {
    let mut best = 0;
    for entry in std::fs::read_dir("/sys/devices/system/cpu/cpufreq").ok()?.flatten() {
        let path = entry.path().join("scaling_cur_freq");
        if let Some(khz) = read(path.to_str()?).and_then(|value| value.parse::<u64>().ok()) {
            best = best.max(khz / 1000);
        }
    }
    (best > 0).then_some(best)
}

fn memory_kb(key: &str) -> Option<u64> {
    read("/proc/meminfo")?.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name != key { return None; }
        value.split_whitespace().next()?.parse().ok()
    })
}

fn storage_usage() -> Option<(u64, u64, u64)> {
    use std::ffi::CString;
    let path = CString::new("/storage").ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } != 0 { return None; }
    let total = stat.f_blocks.saturating_mul(stat.f_frsize);
    let available = stat.f_bavail.saturating_mul(stat.f_frsize);
    let used = total.saturating_sub(available);
    Some((used, total, if total > 0 { used * 100 / total } else { 0 }))
}

fn system_snapshot() -> String {
    let model = read("/proc/device-tree/model").unwrap_or_default();
    let hardware = os_release("HW_CPU").unwrap_or_default();
    let kernel = read("/proc/sys/kernel/osrelease").map(|release| format!("Linux {release} ({})", std::env::consts::ARCH)).unwrap_or_default();
    let os = match (os_release("OS_NAME"), os_release("OS_VERSION")) {
        (Some(name), Some(version)) => format!("{name} {version}"),
        (Some(name), None) => name,
        _ => os_release("PRETTY_NAME").unwrap_or_default(),
    };
    let temperature = read("/sys/class/thermal/thermal_zone0/temp")
        .and_then(|value| value.parse::<u64>().ok())
        .map(|milli| format!("{}C", milli / 1000))
        .unwrap_or_default();
    let frequency = cpu_frequency_mhz().map(|mhz| format!("{mhz}MHz")).unwrap_or_default();
    let ram_total = memory_kb("MemTotal");
    let ram_available = memory_kb("MemAvailable");
    let storage = storage_usage();
    let ram_variant = if std::path::Path::new("/usr/share/bootloader/H700_DDR4_u-boot-sunxi-with-spl.bin").exists() {
        "H700 DDR4"
    } else {
        ""
    };
    let display = read("/sys/class/graphics/fb0/modes")
        .and_then(|value| {
            value.lines().next().map(|line| {
                line.trim_start_matches("U:")
                    .split('p')
                    .next()
                    .unwrap_or("")
                    .replace('x', "×")
            })
        })
        .filter(|value| !value.is_empty())
        .or_else(|| {
            read("/sys/class/graphics/fb0/virtual_size")
                .map(|value| value.lines().next().unwrap_or("").replace(',', "×").replace('x', "×"))
        })
        .or_else(|| {
            std::fs::read_dir("/sys/class/drm").ok().and_then(|entries| {
                entries.flatten().find_map(|entry| {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.contains("DSI") { return None; }
                    read(path.join("modes").to_str()?)
                })
            }).map(|value| value.lines().next().unwrap_or("").replace('x', "×"))
        })
        .unwrap_or_default();
    let field = |key: &str, value: String| format!("{key}\t{}\n", value.replace('\t', " ").replace('\n', " "));
    let mut out = String::new();
    out.push_str(&field("model", model));
    out.push_str(&field("hardware", hardware.clone()));
    out.push_str(&field("cpu", hardware));
    out.push_str(&field("ramVariant", ram_variant.into()));
    if let Some(value) = os_release("OS_NAME") { out.push_str(&field("osName", value)); }
    if let Some(value) = os_release("OS_VERSION") { out.push_str(&field("osVersion", value)); }
    out.push_str(&field("os", os));
    out.push_str(&field("kernel", kernel));
    out.push_str(&field("temperature", temperature));
    out.push_str(&field("frequency", frequency));
    if let Some(value) = ram_available { out.push_str(&field("ramAvailableKb", value.to_string())); }
    if let Some(value) = ram_total { out.push_str(&field("ramTotalKb", value.to_string())); }
    if let Some((used, total, percent)) = storage {
        out.push_str(&field("storageUsed", format!("{:.1}G", used as f64 / 1_073_741_824.0)));
        out.push_str(&field("storageTotal", format!("{:.1}G", total as f64 / 1_073_741_824.0)));
        out.push_str(&field("storagePercent", percent.to_string()));
    }
    out.push_str(&field("display", display));
    out.push_str(&field("runtime", "PocketJS".into()));
    out
}


// ── Games library ───────────────────────────────────────────────────────────
// The scanner is our own service (scripts/pocketshell-games); the render thread
// only reads its JSON catalogue so a big library never stalls the UI.
const GAMES_LIBRARY: &str = "/storage/pocketshell/library/library.json";

fn games_library() -> String {
    std::fs::read_to_string(GAMES_LIBRARY).unwrap_or_else(|_| "{\"total\":0,\"systems\":[]}".into())
}

fn games_scan() -> String {
    let scanner = ["/usr/bin/pocketshell-games", "/storage/dev/usr/bin/pocketshell-games"]
        .iter()
        .find(|candidate| std::path::Path::new(candidate).exists())
        .copied()
        .unwrap_or("/usr/bin/pocketshell-games");
    match run(scanner, &["scan"]) {
        Some(text) => {
            // Nudge the watcher so its cached signature also refreshes.
            let _ = run("pkill", &["-USR1", "-f", "pocketshell-games watch"]);
            text.trim().to_string()
        }
        None => "Scan failed".into(),
    }
}

fn games_launch(path: &str) -> String {
    let rom = std::path::Path::new(path);
    if !rom.is_file() {
        return "ROM not found".into();
    }
    let player = ["/storage/dev/usr/bin/pocketshell-play", "/usr/bin/pocketshell-play"]
        .iter()
        .find(|candidate| std::path::Path::new(candidate).exists())
        .copied();
    let Some(player) = player else {
        return "No player configured".into();
    };
    match run(
        "systemd-run",
        &["--unit=pocketshell-game", "--collect", "--no-block", player, path],
    ) {
        Some(_) => "Starting game".into(),
        None => "Unable to start player".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_accepts_only_supported_values() {
        assert_eq!(language_command(&["LANGUAGE", "ja"]), "ja");
        assert_eq!(language_command(&["LANGUAGE", "xx"]), "Invalid language");
    }

    #[test]
    fn missing_library_is_empty_valid_json() {
        let value = games_library();
        assert!(value.starts_with('{'));
        assert!(value.contains("total"));
    }

    #[test]
    fn launch_rejects_missing_path() {
        assert_eq!(games_launch("/definitely/not/a/rom"), "ROM not found");
    }
}

pub fn command(value: &str) -> String {
    let fields: Vec<&str> = value.split('\t').collect();
    match fields.as_slice() {
        ["LANGUAGE"] | ["LANGUAGE", _] => language_command(&fields),
        ["ROMS"] => games_library(),
        ["ROMSCAN"] => games_scan(),
        ["ROMLAUNCH", path] => games_launch(path),
        ["SYSTEM"] => system_snapshot(),
        ["STATE", key] => match *key {
            "brightness" => command("LEVEL\tbrightness"),
            "volume" => command("LEVEL\tvolume"),
            "wifi" => command("LEVEL\twifi"),
            "bluetooth" => std::fs::read_to_string("/storage/.config/system/configs/system.cfg").ok().and_then(|text| text.lines().find_map(|line| line.trim().strip_prefix("controllers.bluetooth.enabled=").map(str::to_owned))).unwrap_or_else(|| "unknown".into()),
            "mtp" => run("systemctl", &["is-active", "simple-mtpfs.service"]).map(|value| if value.trim() == "active" { "1".into() } else { "0".into() }).unwrap_or_else(|| "0".into()),
            "timeout" => std::fs::read_to_string("/storage/.config/pocketjs/runtime.conf").ok().and_then(|text| {
                let mut section = false;
                text.lines().find_map(|line| { let line = line.trim(); if line.starts_with('[') && line.ends_with(']') { section = line == "[sleep]"; return None; } if section { line.split_once('=').filter(|(key, _)| key.trim() == "timeout").map(|(_, value)| value.trim().to_owned()) } else { None } })
            }).unwrap_or_else(|| "30m".into()),
            key => run("rocknix-services", &["status", &format!("{key}.enabled")]).map(|v| v.trim().split('=').nth(1).unwrap_or("unknown").to_owned()).unwrap_or_else(|| "unknown".into()),
        },
        ["LEVEL", "brightness"] => {
            if let Some(raw) = read("/sys/class/disp/disp/attr/enhance_bright") {
                if let Some(value) = raw.split_whitespace().next().and_then(|word| word.parse::<u32>().ok()) {
                    return value.min(100).to_string();
                }
            }
            if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let read = |name| std::fs::read_to_string(path.join(name)).ok()?.trim().parse::<u32>().ok();
                    if let (Some(current), Some(max)) = (read("brightness"), read("max_brightness")) {
                        if max > 0 { return (current * 100 / max).to_string(); }
                    }
                }
            }
            "unknown".into()
        }
        // Real battery percentage from the fuel gauge (0-100), or "unknown".
        ["LEVEL", "battery"] => {
            for name in ["battery", "BAT0", "BAT1", "rk817-battery", "cw2015-battery"] {
                let base = std::path::Path::new("/sys/class/power_supply").join(name);
                if let Ok(raw) = std::fs::read_to_string(base.join("capacity")) {
                    if let Ok(level) = raw.trim().parse::<u32>() {
                        return level.min(100).to_string();
                    }
                }
            }
            // Fall back to any supply that exposes a capacity.
            if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
                for entry in entries.flatten() {
                    if let Ok(raw) = std::fs::read_to_string(entry.path().join("capacity")) {
                        if let Ok(level) = raw.trim().parse::<u32>() {
                            return level.min(100).to_string();
                        }
                    }
                }
            }
            "unknown".into()
        }
        // First IPv4 address of an interface that is up and not loopback: "iface|addr" or "--".
        ["LEVEL", "ip"] => {
            if let Some(text) = run("ip", &["-o", "-4", "addr", "show"]) {
                for line in text.lines() {
                    let f: Vec<&str> = line.split_whitespace().collect();
                    // inet 203.0.113.5/24 brd ... scope global eth0
                    if f.len() < 4 || f.get(2) != Some(&"inet") { continue; }
                    let iface = *f.last().unwrap_or(&"");
                    if iface == "lo" { continue; }
                    if line.contains("scope host") { continue; }
                    let addr = f[3].split('/').next().unwrap_or("");
                    if addr.is_empty() { continue; }
                    return format!("{}|{}", iface, addr);
                }
            }
            "--".into()
        }
        // "on" when a wireless interface is up and associated, else "off".
        ["LEVEL", "wifi"] => {
            let associated = std::fs::read_dir("/sys/class/net")
                .map(|entries| {
                    entries.flatten().any(|e| {
                        let inet = e.path().join("wireless");
                        let oper = std::fs::read_to_string(e.path().join("operstate"))
                            .map(|s| s.trim() == "up")
                            .unwrap_or(false);
                        inet.exists() && oper
                    })
                })
                .unwrap_or(false);
            if associated { "on".into() } else { "off".into() }
        }
        ["LEVEL", "volume"] => {
            for (program, args) in [("amixer", vec!["get", "Master"]), ("pactl", vec!["get-sink-volume", "@DEFAULT_SINK@"])] {
                let Some(text) = run(program, &args) else { continue };
                for word in text.split_whitespace() {
                    let token = word.trim_matches(|c| c == '[' || c == ']');
                    if let Some(n) = token.strip_suffix('%').and_then(|s| s.parse::<u32>().ok()) { return n.min(100).to_string(); }
                }
            }
            "unknown".into()
        }
        ["LAUNCH", app] if ["disk", "xmplayer"].contains(app) => {
            match run("systemd-run", &["--unit=pocketjs-launcher", "--collect", "--no-block", "/usr/bin/pocketjs-native-launch", app]) {
                Some(_) => "Opening application".into(),
                None => "Unable to start application".into(),
            }
        }
        ["CONTROL", control, value] if ["volume", "brightness", "wifi", "bluetooth"].contains(control) => {
            let valid = if ["volume", "brightness"].contains(control) { value.parse::<u8>().is_ok_and(|n| n <= 100) } else { ["on", "off"].contains(value) };
            if !valid { return "Invalid setting".into(); }
            let args: &[&str] = if *control == "brightness" { &["--command", "set", "--key", "brightness", "--value", value] } else { &[control, value] };
            match run(if *control == "brightness" { "rocknix-settings" } else { "heya-control" }, args) {
                Some(_) => "Setting updated".into(),
                None => "Setting failed".into(),
            }
        }
        ["SERVICE", key] if ["adbd.enabled", "adbusb.enabled", "ssh.enabled", "avahi.enabled", "ftp.enabled", "http.enabled", "rsyncd.enabled", "samba.enabled", "syncthing.enabled", "telnet.enabled", "pocketshell-dev.enabled"].contains(key) => {
            match run("rocknix-services", &["toggle", key]) {
                Some(text) => text.trim().to_string(),
                None => "Service change failed".into(),
            }
        }
        ["SLEEP", timeout] => {
            let path = std::path::Path::new("/storage/.config/pocketjs/runtime.conf");
            let mut content = std::fs::read_to_string(path).unwrap_or_default();
            let mut in_section = false;
            let mut replaced = false;
            let mut new_lines: Vec<String> = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_section = trimmed == "[sleep]";
                    new_lines.push(line.to_string());
                    continue;
                }
                if in_section && trimmed.starts_with("timeout") {
                    new_lines.push(format!("timeout = {}", timeout));
                    replaced = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }
            if !replaced {
                if !content.ends_with('\n') { new_lines.push(String::new()); }
                if !in_section {
                    new_lines.push("[sleep]".to_string());
                }
                new_lines.push(format!("timeout = {}", timeout));
            }
            match std::fs::write(path, new_lines.join("\n") + "\n") {
                Ok(_) => {
                    // Reload the daemon if it is running.
                    let _ = run("systemctl", &["try-restart", "pocketjs-sleep.service"]);
                    format!("Screen timeout set to {}", timeout)
                }
                Err(_) => "Failed to save timeout".into(),
            }
        }
        _ => "Unknown launcher action".into(),
    }
}
