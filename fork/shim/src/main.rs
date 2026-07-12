#![windows_subsystem = "windows"]

//! ByteLauncher compatibility shim.
//!
//! The installer renames the original `Modrinth App.exe` to
//! `Modrinth App.old.exe` and drops this tiny launcher in its place as
//! `Modrinth App.exe`. That way existing Start Menu / desktop / pinned
//! shortcuts (which all point at `Modrinth App.exe`) keep working and now open
//! ByteLauncher. All command-line arguments are forwarded, so `modrinth://`
//! deep links and `.mrpack` file associations still launch the app correctly.

use std::process::Command;

fn main() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let target = dir.join("ByteLauncher.exe");

    // args_os so a non-Unicode argument can't panic the shim (which, with
    // panic="abort", would just make the shortcut do nothing).
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let _ = Command::new(target).args(args).spawn();
}
