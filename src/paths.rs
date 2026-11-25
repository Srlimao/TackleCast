use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static PROBE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Returns the portable directory used in normal installs (next to the executable in release,
/// or the working directory in debug builds).
pub fn portable_base_dir() -> PathBuf {
    let exe_parent = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    let cwd = std::env::current_dir().ok();

    if cfg!(debug_assertions) {
        cwd.or(exe_parent).unwrap_or_else(|| PathBuf::from("."))
    } else {
        exe_parent.or(cwd).unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Returns the per-user configuration/data fallback directory for the current OS.
pub fn user_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        windows_user_data_dir_from(|key| std::env::var_os(key))
    }
    #[cfg(not(target_os = "windows"))]
    {
        unix_user_data_dir_from(|key| std::env::var_os(key))
    }
}

#[cfg(any(target_os = "windows", test))]
fn windows_user_data_dir_from<F>(get: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let non_empty = |key: &str| get(key).filter(|value| !value.to_string_lossy().trim().is_empty());

    if let Some(dir) = non_empty("APPDATA").or_else(|| non_empty("LOCALAPPDATA")) {
        return Some(PathBuf::from(dir).join("TackleCast"));
    }
    non_empty("USERPROFILE").map(|dir| {
        PathBuf::from(dir)
            .join("AppData")
            .join("Roaming")
            .join("TackleCast")
    })
}

#[cfg(any(not(target_os = "windows"), test))]
fn unix_user_data_dir_from<F>(get: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let non_empty = |key: &str| get(key).filter(|value| !value.to_string_lossy().trim().is_empty());

    if let Some(xdg) = non_empty("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg);
        if path.is_absolute() {
            return Some(path.join("tacklecast"));
        }
    }
    non_empty("HOME").map(|dir| PathBuf::from(dir).join(".config").join("tacklecast"))
}

/// Probes whether files can be created inside `dir` without leaving probe artifacts behind.
///
/// Uses an actual file creation probe because Windows directory ACLs (e.g. `Program Files`)
/// are not reliably represented by `metadata().permissions().readonly()`.
fn dir_is_writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let seq = PROBE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let probe = dir.join(format!(
        ".tacklecast-write-probe-{}-{seq}",
        std::process::id()
    ));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn resolve_data_dir_with<F>(portable: &Path, fallback: Option<PathBuf>, is_writable: F) -> PathBuf
where
    F: Fn(&Path) -> bool,
{
    if is_writable(portable) {
        portable.to_path_buf()
    } else if let Some(fb) = fallback {
        fb
    } else {
        portable.to_path_buf()
    }
}

/// Returns the active writable directory for TackleCast settings, logs, and recordings.
pub fn data_dir() -> &'static Path {
    DATA_DIR
        .get_or_init(|| {
            resolve_data_dir_with(&portable_base_dir(), user_data_dir(), dir_is_writable)
        })
        .as_path()
}

pub fn settings_path() -> PathBuf {
    data_dir().join("tacklecast_settings.json")
}

pub fn portable_settings_path() -> PathBuf {
    portable_base_dir().join("tacklecast_settings.json")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn recordings_dir() -> PathBuf {
    data_dir().join("recordings")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "tacklecast-paths-test-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn writable_portable_dir_is_preferred_over_fallback() {
        let portable = PathBuf::from("portable_dir");
        let fallback = PathBuf::from("user_fallback_dir");

        assert_eq!(
            resolve_data_dir_with(&portable, Some(fallback.clone()), |_| true),
            portable
        );
        assert_eq!(
            resolve_data_dir_with(&portable, Some(fallback), |_| false),
            PathBuf::from("user_fallback_dir")
        );
        assert_eq!(resolve_data_dir_with(&portable, None, |_| false), portable);
    }

    #[test]
    fn dir_is_writable_probes_without_leaving_files() {
        let dir = temp_test_dir("writable");
        assert!(dir_is_writable(&dir));
        let leftover: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".tacklecast-write-probe-")
            })
            .collect();
        assert!(leftover.is_empty(), "probe file should be removed");

        let blocking_file = dir.join("not_a_dir");
        fs::write(&blocking_file, b"x").unwrap();
        assert!(!dir_is_writable(&blocking_file.join("nested")));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_and_unix_user_data_dir_env_precedence() {
        let win_full = windows_user_data_dir_from(|key| match key {
            "APPDATA" => Some(OsString::from(r"C:\Users\alice\AppData\Roaming")),
            "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\alice\AppData\Local")),
            "USERPROFILE" => Some(OsString::from(r"C:\Users\alice")),
            _ => None,
        });
        assert_eq!(
            win_full,
            Some(PathBuf::from(r"C:\Users\alice\AppData\Roaming").join("TackleCast"))
        );

        let win_local = windows_user_data_dir_from(|key| match key {
            "APPDATA" => Some(OsString::from("   ")),
            "LOCALAPPDATA" => Some(OsString::from(r"C:\Users\alice\AppData\Local")),
            "USERPROFILE" => Some(OsString::from(r"C:\Users\alice")),
            _ => None,
        });
        assert_eq!(
            win_local,
            Some(PathBuf::from(r"C:\Users\alice\AppData\Local").join("TackleCast"))
        );

        let win_profile = windows_user_data_dir_from(|key| match key {
            "USERPROFILE" => Some(OsString::from(r"C:\Users\alice")),
            _ => None,
        });
        assert_eq!(
            win_profile,
            Some(
                PathBuf::from(r"C:\Users\alice")
                    .join("AppData")
                    .join("Roaming")
                    .join("TackleCast")
            )
        );

        #[cfg(not(windows))]
        let abs_xdg = "/home/alice/.custom-config";
        #[cfg(windows)]
        let abs_xdg = r"C:\custom-config";

        let unix_xdg = unix_user_data_dir_from(|key| match key {
            "XDG_CONFIG_HOME" => Some(OsString::from(abs_xdg)),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(unix_xdg, Some(PathBuf::from(abs_xdg).join("tacklecast")));

        let unix_relative_xdg = unix_user_data_dir_from(|key| match key {
            "XDG_CONFIG_HOME" => Some(OsString::from("relative/config")),
            "HOME" => Some(OsString::from("/home/alice")),
            _ => None,
        });
        assert_eq!(
            unix_relative_xdg,
            Some(
                PathBuf::from("/home/alice")
                    .join(".config")
                    .join("tacklecast")
            )
        );
    }
}
