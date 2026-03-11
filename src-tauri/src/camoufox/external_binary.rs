use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// Get the path to the external camoufox-js binary if it exists.
/// On macOS, this is typically in ~/Library/Caches/camoufox/
pub fn get_external_camoufox_path() -> Option<PathBuf> {
  let base_dirs = BaseDirs::new()?;
  let cache_dir = base_dirs.cache_dir();
  let camoufox_cache_dir = cache_dir.join("camoufox");

  log::info!(
    "Checking for external Camoufox binary in: {:?}",
    camoufox_cache_dir
  );

  if !camoufox_cache_dir.exists() {
    return None;
  }

  // Search for a camoufox binary inside the cache directory
  // camoufox-js typically stores binaries in subdirectories like camoufox-<version>/

  // Scan subdirectories
  if let Ok(entries) = std::fs::read_dir(&camoufox_cache_dir) {
    let mut versions = Vec::new();
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        let binary_path = if cfg!(windows) {
          path.join("camoufox.exe")
        } else {
          path.join("camoufox")
        };

        if is_executable(&binary_path) {
          versions.push(binary_path);
        }
      }
    }

    // Return the latest one (lexicographical for simplicity, or just first found)
    if !versions.is_empty() {
      versions.sort();
      let latest = versions.last().cloned();
      log::info!("Found external Camoufox binary: {:?}", latest);
      return latest;
    }
  }

  // Check if it's directly in the cache dir (less likely)
  let direct_binary = if cfg!(windows) {
    camoufox_cache_dir.join("camoufox.exe")
  } else {
    camoufox_cache_dir.join("camoufox")
  };

  if is_executable(&direct_binary) {
    log::info!(
      "Found external Camoufox binary (direct): {:?}",
      direct_binary
    );
    return Some(direct_binary);
  }

  None
}

fn is_executable(path: &Path) -> bool {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = path.metadata() {
      return metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;
    }
  }
  #[cfg(windows)]
  {
    if let Ok(metadata) = path.metadata() {
      return metadata.is_file() && path.extension().and_then(|s| s.to_str()) == Some("exe");
    }
  }
  false
}
