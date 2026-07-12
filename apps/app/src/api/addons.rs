//! Plugin ("addons") system for the ByteLauncher fork.
//!
//! Plugins live in `<settings_dir>/plugins/<id>/` as a folder containing a
//! `manifest.json` plus the referenced `.js`/`.css` files. Enabled state is
//! persisted separately in `<settings_dir>/plugins/enabled.json` so upgrading
//! the built-in plugins never clobbers the user's on/off choices.
//!
//! The frontend loader (`apps/app-frontend/src/plugins/plugin-loader.js`) calls
//! [`read_plugins`] once on startup and injects each enabled plugin's CSS/JS.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::plugin::TauriPlugin;
use tokio::sync::OnceCell;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("addons")
        .invoke_handler(tauri::generate_handler![
            read_plugins,
            set_plugin_enabled,
            get_plugins_dir,
            fork_apply_update,
            fork_uninstall,
            set_hosting_webview,
        ])
        .build()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    js: Option<String>,
    #[serde(default)]
    css: Option<String>,
    #[serde(default)]
    enabled_by_default: bool,
    #[serde(default)]
    builtin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginData {
    id: String,
    name: String,
    description: String,
    version: String,
    author: String,
    enabled: bool,
    builtin: bool,
    js: Option<String>,
    css: Option<String>,
}

async fn plugins_dir() -> crate::api::Result<PathBuf> {
    let state = theseus::State::get().await?;
    let dir = state.directories.settings_dir.join("plugins");
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

async fn read_enabled_map() -> crate::api::Result<HashMap<String, bool>> {
    let path = plugins_dir().await?.join("enabled.json");
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => Ok(serde_json::from_str(&contents).unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(e.into()),
    }
}

async fn write_enabled_map(map: &HashMap<String, bool>) -> crate::api::Result<()> {
    let path = plugins_dir().await?.join("enabled.json");
    let json = serde_json::to_string_pretty(map)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    tokio::fs::write(&path, json).await?;
    Ok(())
}

#[tauri::command]
pub async fn read_plugins() -> crate::api::Result<Vec<PluginData>> {
    ensure_seeded().await;
    cleanup_stale_update_files().await;

    let dir = plugins_dir().await?;
    let enabled_map = read_enabled_map().await?;

    let mut out = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let path = entry.path();
        let manifest_str = match tokio::fs::read_to_string(path.join("manifest.json")).await {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let manifest: Manifest = match serde_json::from_str(&manifest_str) {
            Ok(manifest) => manifest,
            Err(e) => {
                tracing::warn!("Skipping invalid plugin manifest in {path:?}: {e}");
                continue;
            }
        };

        let js = match &manifest.js {
            Some(file) if !file.is_empty() => {
                tokio::fs::read_to_string(path.join(file)).await.ok()
            }
            _ => None,
        };
        let css = match &manifest.css {
            Some(file) if !file.is_empty() => {
                tokio::fs::read_to_string(path.join(file)).await.ok()
            }
            _ => None,
        };

        let enabled = enabled_map
            .get(&manifest.id)
            .copied()
            .unwrap_or(manifest.enabled_by_default);

        out.push(PluginData {
            name: if manifest.name.is_empty() {
                manifest.id.clone()
            } else {
                manifest.name.clone()
            },
            id: manifest.id,
            description: manifest.description,
            version: manifest.version,
            author: manifest.author,
            enabled,
            builtin: manifest.builtin,
            js,
            css,
        });
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

#[tauri::command]
pub async fn set_plugin_enabled(id: String, enabled: bool) -> crate::api::Result<()> {
    let mut map = read_enabled_map().await?;
    map.insert(id, enabled);
    write_enabled_map(&map).await
}

#[tauri::command]
pub async fn get_plugins_dir() -> crate::api::Result<String> {
    Ok(plugins_dir().await?.to_string_lossy().to_string())
}

fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}

/// Downloads a new `ByteLauncher.exe` from `download_url` (which must be an
/// HTTPS GitHub URL), verifies it against `expected_sha256` when provided,
/// swaps it in for the currently running executable (keeping the old one as
/// `*.old.exe`), and restarts. The swap is guarded so the app is never left
/// without a working executable.
#[tauri::command]
pub async fn fork_apply_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    download_url: String,
    expected_sha256: Option<String>,
) -> crate::api::Result<()> {
    use sha2::{Digest, Sha256};
    use tauri_plugin_http::reqwest;

    // This command is reachable from plugin JS, so only allow HTTPS downloads
    // from GitHub's own release hosts — otherwise an arbitrary URL would be a
    // code-execution vector.
    let url = reqwest::Url::parse(&download_url).map_err(io_other)?;
    if url.scheme() != "https" {
        return Err(io_other("update URL must be HTTPS").into());
    }
    match url.host_str() {
        Some("github.com")
        | Some("objects.githubusercontent.com")
        | Some("release-assets.githubusercontent.com") => {}
        _ => return Err(io_other("update URL host is not allowed").into()),
    }

    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| io_other("executable has no parent directory"))?
        .to_path_buf();
    let staged = dir.join("ByteLauncher.update.exe");
    let backup = dir.join("ByteLauncher.old.exe");

    let bytes = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "ByteLauncher-Updater")
        .send()
        .await
        .map_err(io_other)?
        .error_for_status()
        .map_err(io_other)?
        .bytes()
        .await
        .map_err(io_other)?;

    // Must be a Windows executable ("MZ") of a plausible size.
    if bytes.len() < 5_000_000 || !bytes.starts_with(b"MZ") {
        return Err(io_other("downloaded file is not a valid Windows executable").into());
    }

    // Verify the GitHub-published SHA-256 digest when we have one.
    if let Some(expected) = expected_sha256
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let actual: String = Sha256::digest(bytes.as_ref())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(io_other("update failed checksum verification").into());
        }
    }

    tokio::fs::write(&staged, &bytes).await?;

    let _ = tokio::fs::remove_file(&backup).await;
    tokio::fs::rename(&exe, &backup).await?;
    if let Err(e) = tokio::fs::rename(&staged, &exe).await {
        // `backup` holds the only working binary — restore it no matter what.
        if tokio::fs::rename(&backup, &exe).await.is_err() {
            let _ = tokio::fs::copy(&backup, &exe).await;
        }
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(e.into());
    }
    // Never restart into a missing executable.
    if !tokio::fs::try_exists(&exe).await.unwrap_or(false) {
        let _ = tokio::fs::copy(&backup, &exe).await;
        return Err(io_other("update left no executable in place").into());
    }

    app.restart();
}

/// Removes the backup left by a previous self-update (best effort). The
/// transient `*.update.exe` is intentionally left alone here so this cannot
/// race an in-progress update.
async fn cleanup_stale_update_files() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = tokio::fs::remove_file(dir.join("ByteLauncher.old.exe")).await;
        }
    }
}

/// Reverts ByteLauncher back to the Modrinth App by launching the installer's
/// uninstaller — which restores `Modrinth App.exe` from the `.old.exe` backup
/// and removes ByteLauncher's own files — then quits so it can finish. Only
/// works for installs made with the ByteLauncher installer (which drops the
/// uninstaller next to the exe); a raw-exe drop-in has no uninstaller.
#[tauri::command]
pub async fn fork_uninstall<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> crate::api::Result<()> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| io_other("executable has no parent directory"))?;
    let uninstaller = dir.join("ByteLauncher-uninstall.exe");

    if !uninstaller.exists() {
        return Err(io_other(
            "Uninstaller not found — ByteLauncher wasn't installed with the ByteLauncher installer. To revert manually, close ByteLauncher and rename \"Modrinth App.old.exe\" back to \"Modrinth App.exe\".",
        )
        .into());
    }

    std::process::Command::new(&uninstaller)
        .spawn()
        .map_err(io_other)?;

    // Quit so the uninstaller can replace the running executable.
    app.exit(0);

    Ok(())
}

/// Shows or hides the ByteBuilders hosting panel as a **native child webview**
/// positioned over the hosting page's content area. Rendering the panel in a
/// real webview (rather than an `<iframe>`) makes it a top-level document, so
/// the panel's `X-Frame-Options`/`frame-ancestors` restrictions don't apply and
/// it renders with no changes needed on the panel side. Bounds are physical
/// pixels supplied by the frontend (`getBoundingClientRect()` × devicePixelRatio).
#[tauri::command]
pub async fn set_hosting_webview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    visible: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> crate::api::Result<()> {
    use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewUrl};

    const HOSTING_URL: &str = "https://panel.bytebuilders.co.za";
    const LABEL: &str = "hosting-webview";

    if !visible {
        if let Some(webview) = app.webviews().get(LABEL) {
            webview.hide().ok();
            webview
                .set_position(PhysicalPosition::new(-4000.0, -4000.0))
                .ok();
        }
        return Ok(());
    }

    let position = PhysicalPosition::new(x, y);
    let size = PhysicalSize::new(width.max(1.0), height.max(1.0));

    if let Some(webview) = app.webviews().get(LABEL) {
        webview.set_position(position).ok();
        webview.set_size(size).ok();
        webview.show().ok();
    } else if let Some(window) = app.get_window("main") {
        let webview = window.add_child(
            tauri::webview::WebviewBuilder::new(
                LABEL,
                WebviewUrl::External(HOSTING_URL.parse().unwrap()),
            ),
            position,
            size,
        )?;
        webview.show().ok();
    }

    Ok(())
}

/// Whether a plugin is currently enabled. Used natively (e.g. by the ads
/// module) to honor plugin toggles without a frontend round-trip. Only the
/// ads module calls this, and that module is not built on Linux.
#[cfg(not(target_os = "linux"))]
pub async fn is_plugin_enabled(id: &str) -> bool {
    ensure_seeded().await;

    if let Ok(map) = read_enabled_map().await {
        if let Some(&value) = map.get(id) {
            return value;
        }
    }

    let Ok(dir) = plugins_dir().await else {
        return false;
    };
    match tokio::fs::read_to_string(dir.join(id).join("manifest.json")).await {
        Ok(contents) => serde_json::from_str::<Manifest>(&contents)
            .map(|manifest| manifest.enabled_by_default)
            .unwrap_or(false),
        Err(_) => false,
    }
}

struct BuiltinFile {
    name: &'static str,
    content: &'static str,
    /// Never overwrite once the user has one (e.g. their custom CSS).
    preserve: bool,
}

struct Builtin {
    id: &'static str,
    files: &'static [BuiltinFile],
}

const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "hide-ads",
        files: &[
            BuiltinFile {
                name: "manifest.json",
                content: include_str!("builtin_plugins/hide-ads/manifest.json"),
                preserve: false,
            },
            BuiltinFile {
                name: "styles.css",
                content: include_str!("builtin_plugins/hide-ads/styles.css"),
                preserve: false,
            },
        ],
    },
    Builtin {
        id: "multi-launch",
        files: &[BuiltinFile {
            name: "manifest.json",
            content: include_str!("builtin_plugins/multi-launch/manifest.json"),
            preserve: false,
        }],
    },
    Builtin {
        id: "custom-css",
        files: &[
            BuiltinFile {
                name: "manifest.json",
                content: include_str!("builtin_plugins/custom-css/manifest.json"),
                preserve: false,
            },
            BuiltinFile {
                name: "user.css",
                content: include_str!("builtin_plugins/custom-css/user.css"),
                preserve: true,
            },
        ],
    },
];

static SEEDED: OnceCell<()> = OnceCell::const_new();

async fn ensure_seeded() {
    // get_or_try_init makes every concurrent caller await the same seeding
    // future, and only marks the cell initialized on success (so a failed seed
    // is retried on the next call). This avoids reading a half-written dir.
    let result = SEEDED
        .get_or_try_init(|| async { seed_builtin_plugins().await })
        .await;
    if let Err(e) = result {
        tracing::warn!("Failed to seed built-in plugins: {e}");
    }
}

async fn seed_builtin_plugins() -> crate::api::Result<()> {
    let dir = plugins_dir().await?;
    for builtin in BUILTINS {
        let plugin_dir = dir.join(builtin.id);
        tokio::fs::create_dir_all(&plugin_dir).await?;
        for file in builtin.files {
            let file_path = plugin_dir.join(file.name);
            if file.preserve && tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
                continue;
            }
            tokio::fs::write(&file_path, file.content).await?;
        }
    }
    // multi-launch v1.x seeded an index.js; it is a native feature now.
    let _ = tokio::fs::remove_file(dir.join("multi-launch").join("index.js")).await;
    Ok(())
}
