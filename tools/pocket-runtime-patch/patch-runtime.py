#!/usr/bin/env python3
"""Add PocketShell's terminal and bounded Library bridge to pinned PocketJS."""
from pathlib import Path
import sys

root = Path(sys.argv[1])
script_dir = Path(__file__).resolve().parent
guest = root / "engine/guest-rquickjs/src/lib.rs"
(root / "engine/guest-rquickjs/src/pocketshell_terminal.rs").write_text((script_dir / "pocketshell-terminal.rs").read_text())
(root / "engine/guest-rquickjs/src/pocketshell_library.rs").write_text((script_dir / "pocketshell-library.rs").read_text())
(root / "engine/guest-rquickjs/src/pocketshell_launcher.rs").write_text((script_dir / "pocketshell-launcher.rs").read_text())

cargo = root / "engine/guest-rquickjs/Cargo.toml"
cargo_text = cargo.read_text()
if 'libc = "0.2.191"' not in cargo_text:
    cargo.write_text(cargo_text.replace('rquickjs = "0.12"', 'rquickjs = "0.12"\nlibc = "0.2.191"', 1))

source = guest.read_text()

def replace(old: str, new: str) -> None:
    global source
    if new in source:
        return
    if old not in source:
        raise SystemExit(f"PocketShell patch anchor missing: {old[:60]!r}")
    source = source.replace(old, new, 1)

replace(
    "use rquickjs::{\n    function::Rest, Array, ArrayBuffer, Coerced, Context, Ctx, Exception, FromJs, Function, IntoJs,\n    Object, Runtime, Value,\n};",
    "mod pocketshell_launcher;\nmod pocketshell_library;\nmod pocketshell_terminal;\n\nuse pocketshell_terminal::PocketShell;\nuse rquickjs::{\n    function::Rest, Array, ArrayBuffer, Coerced, Context, Ctx, Exception, FromJs, Function, IntoJs,\n    Object, Runtime, Value,\n};\nuse std::sync::{Arc, Mutex};",
)
replace("    last_error: Option<String>,\n}", "    last_error: Option<String>,\n    pocketshell: Arc<Mutex<PocketShell>>,\n}")
replace("            last_error: None,\n        }", "            last_error: None,\n            pocketshell: Arc::new(Mutex::new(PocketShell::new())),\n        }")
replace(
    "        let result = context.with(|ctx| mount_ui(&ctx, ops, environment, self.profile));",
    """        let shell = self.pocketshell.clone();
        let result: rquickjs::Result<()> = context.with(|ctx| {
            mount_ui(&ctx, ops, environment, self.profile)?;
            let ui: Object = ctx.globals().get("ui")?;
            let command_shell = shell.clone();
            ui.set("__pocketshell", Function::new(ctx.clone(), move |command: String| {
                if command.len() > 16384 { return String::new(); }
                command_shell.lock().map(|mut state| state.command(&command)).unwrap_or_default()
            })?)?;
            ui.set("__launcher", Function::new(ctx.clone(), |command: String| pocketshell_launcher::command(&command))?)?;
            ui.set("__library", Function::new(ctx.clone(), |command: String| pocketshell_library::command(&command))?)?;
            let frame_shell = shell.clone();
            ctx.globals().set("__pocketshellFrame", Function::new(ctx.clone(), move || {
                frame_shell.lock().map(|mut state| state.frame()).unwrap_or_default()
            })?)?;
            Ok(())
        });""",
)
replace(
    "        let result = context.with(|ctx| call_frame(&ctx, frame));",
    """        let shell = self.pocketshell.clone();
        let result: rquickjs::Result<()> = context.with(|ctx| {
            let output = shell.lock().map(|mut state| state.frame()).unwrap_or_default();
            if !output.is_empty() {
                let global: Object = ctx.globals().get("globalThis")?;
                global.set("__pocketshellOutput", output)?;
            }
            call_frame(&ctx, frame)
        });""",
)
guest.write_text(source)
