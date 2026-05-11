use crate::browser::{create_browser, BrowserType, ProxySettings};
use crate::camoufox_manager::{CamoufoxConfig, CamoufoxManager};
use crate::cloakbrowser_manager::CloakBrowserConfig;
use crate::downloaded_browsers_registry::DownloadedBrowsersRegistry;
use crate::events;
use crate::platform_browser;
use crate::profile::{BrowserProfile, ProfileManager};
use crate::proxy_manager::PROXY_MANAGER;
use crate::wayfern_manager::{WayfernConfig, WayfernManager};
use directories::BaseDirs;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use sysinfo::ProcessStatus;
use sysinfo::System;
pub struct BrowserRunner {
  base_dirs: BaseDirs,
  pub profile_manager: &'static ProfileManager,
  pub downloaded_browsers_registry: &'static DownloadedBrowsersRegistry,
  auto_updater: &'static crate::auto_updater::AutoUpdater,
  camoufox_manager: &'static CamoufoxManager,
  wayfern_manager: &'static WayfernManager,
}

impl BrowserRunner {
  fn new() -> Self {
    Self {
      base_dirs: BaseDirs::new().expect("Failed to get base directories"),
      profile_manager: ProfileManager::instance(),
      downloaded_browsers_registry: DownloadedBrowsersRegistry::instance(),
      auto_updater: crate::auto_updater::AutoUpdater::instance(),
      camoufox_manager: CamoufoxManager::instance(),
      wayfern_manager: WayfernManager::instance(),
    }
  }

  pub fn instance() -> &'static BrowserRunner {
    &BROWSER_RUNNER
  }

  pub fn get_binaries_dir(&self) -> PathBuf {
    let mut path = self.base_dirs.data_local_dir().to_path_buf();
    path.push(if cfg!(debug_assertions) {
      "FoxiaDev"
    } else {
      "Foxia"
    });
    path.push("binaries");
    path
  }

  /// Get the executable path for a browser profile
  /// This is a common helper to eliminate code duplication across the codebase
  pub fn get_browser_executable_path(
    &self,
    profile: &BrowserProfile,
  ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Create browser instance to get executable path
    let browser_type = crate::browser::BrowserType::from_str(&profile.browser)
      .map_err(|e| format!("Invalid browser type: {e}"))?;
    let browser = crate::browser::create_browser(browser_type);

    // Construct browser directory path: binaries/<browser>/<version>/
    let mut browser_dir = self.get_binaries_dir();
    browser_dir.push(&profile.browser);
    browser_dir.push(&profile.version);

    // Fallback for Camoufox to external camoufox-js binary
    if profile.browser == "camoufox" {
      if let Some(external_path) = crate::camoufox::external_binary::get_external_camoufox_path() {
        if external_path.exists() {
          log::info!(
            "Using external Camoufox binary as primary source: {:?}",
            external_path
          );
          return Ok(external_path);
        }
      }
    }

    // Get platform-specific executable path
    let internal_path_res = browser.get_executable_path(&browser_dir);

    log::info!(
      "Internal path resolution for {}: {:?}",
      profile.browser,
      internal_path_res
    );

    match internal_path_res {
      Ok(path) if path.exists() => {
        log::info!("Found executable at: {:?}", path);
        Ok(path)
      }
      res => {
        // Final fallback for Camoufox
        if profile.browser == "camoufox" {
          if let Some(external_path) =
            crate::camoufox::external_binary::get_external_camoufox_path()
          {
            if external_path.exists() {
              log::info!(
                "Using fallback external Camoufox binary: {:?}",
                external_path
              );
              return Ok(external_path);
            }
          }
        }

        // It didn't exist or was an error - return error now
        match res {
          Ok(path) => {
            let err_msg = format!(
              "Browser executable found at {:?} but it does not exist on disk. Please try downloading it again.",
              path
            );
            log::error!("{}", err_msg);
            Err(err_msg.into())
          }
          Err(e) => {
            let err_msg = format!("Failed to get executable path for {}: {e}", profile.browser);
            log::error!("{}", err_msg);
            Err(err_msg.into())
          }
        }
      }
    }
  }

  /// Resolve the effective upstream proxy settings for a profile.
  /// Prioritizes local proxy_id, then falls back to odoo_proxy.
  pub fn resolve_upstream_proxy(&self, profile: &BrowserProfile) -> Option<ProxySettings> {
    log::info!(
      "Resolving upstream proxy for profile: {} (ID: {})",
      profile.name,
      profile.id
    );
    log::info!("  - proxy_id: {:?}", profile.proxy_id);
    log::info!("  - has odoo_proxy: {}", profile.odoo_proxy.is_some());

    // 1. Give priority to local proxy_id if set
    if let Some(proxy_id) = &profile.proxy_id {
      if let Some(settings) = PROXY_MANAGER.get_proxy_settings_by_id(proxy_id) {
        log::info!(
          "  => Resolved from proxy_id: {}:{}",
          settings.host,
          settings.port
        );
        return Some(settings);
      } else {
        log::warn!(
          "  ! proxy_id set but settings not found in PROXY_MANAGER: {}",
          proxy_id
        );
      }
    }

    // 2. Fallback to odoo_proxy if available (tải động từ Odoo frontend)
    if let Some(odoo_proxy) = &profile.odoo_proxy {
      log::info!("  - Odoo Proxy data: {}:{}", odoo_proxy.ip, odoo_proxy.port);
      let port = match &odoo_proxy.port {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u16,
        serde_json::Value::String(s) => s.parse::<u16>().unwrap_or(0),
        _ => {
          log::warn!("  ! Invalid Odoo proxy port type: {:?}", odoo_proxy.port);
          0
        }
      };

      if port > 0 {
        let settings = ProxySettings {
          proxy_type: odoo_proxy.giaothuc.to_lowercase(),
          host: odoo_proxy.ip.clone(),
          port,
          username: odoo_proxy.tendangnhap.clone(),
          password: odoo_proxy.matkhau.clone(),
        };
        let user_masked = settings
          .username
          .as_ref()
          .map(|u| {
            if u.len() > 3 {
              format!("{}***", &u[..3])
            } else {
              "***".to_string()
            }
          })
          .unwrap_or_else(|| "none".to_string());

        log::info!(
          "  => Resolved from odoo_proxy: {}:{} (user: {}, has_pass: {})",
          settings.host,
          settings.port,
          user_masked,
          settings.password.is_some()
        );
        return Some(settings);
      } else {
        log::warn!("  ! Odoo proxy port is 0 or invalid");
      }
    }

    log::info!("  => Using DIRECT (no proxy)");
    None
  }

  fn normalize_cloak_platform(platform: &str) -> Option<String> {
    let platform = platform.trim().to_ascii_lowercase();
    if platform.is_empty() {
      return None;
    }

    if platform.contains("android") {
      return Some("android".to_string());
    }
    if platform.contains("mac") || platform.contains("darwin") {
      return Some("macos".to_string());
    }
    if platform.contains("win") {
      return Some("windows".to_string());
    }
    if platform.contains("linux") || platform.contains("x11") {
      return Some("linux".to_string());
    }

    None
  }

  fn platform_from_user_agent(user_agent: &str) -> Option<String> {
    Self::normalize_cloak_platform(user_agent)
  }

  fn json_string(value: &Value, key: &str) -> Option<String> {
    value
      .get(key)
      .and_then(|item| item.as_str())
      .map(str::trim)
      .filter(|item| !item.is_empty())
      .map(ToOwned::to_owned)
  }

  fn first_language_from_value(value: &Value) -> Option<String> {
    if let Some(item) = value.as_str() {
      let item = item.trim();
      if item.is_empty() {
        return None;
      }

      if item.starts_with('[') {
        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(item) {
          return parsed
            .into_iter()
            .map(|lang| lang.trim().to_string())
            .find(|lang| !lang.is_empty());
        }
      }

      return Some(item.to_string());
    }

    value.as_array().and_then(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str())
        .map(str::trim)
        .find(|item| !item.is_empty())
        .map(ToOwned::to_owned)
    })
  }

  fn locale_from_fingerprint(fingerprint: &Value) -> Option<String> {
    if let Some(locale) = Self::json_string(fingerprint, "locale:all") {
      return Some(locale);
    }

    if let Some(locale) = Self::json_string(fingerprint, "navigator.language") {
      return Some(locale);
    }

    if let Some(locale) = fingerprint
      .get("languages")
      .and_then(Self::first_language_from_value)
    {
      return Some(locale);
    }

    let language = Self::json_string(fingerprint, "locale:language")
      .or_else(|| Self::json_string(fingerprint, "language"));
    let region = Self::json_string(fingerprint, "locale:region");

    match (language, region) {
      (Some(language), Some(region)) if !language.contains('-') => {
        Some(format!("{language}-{region}"))
      }
      (Some(language), _) => Some(language),
      _ => None,
    }
  }

  fn app_dir_name() -> &'static str {
    if cfg!(debug_assertions) {
      "FoxiaDev"
    } else {
      "Foxia"
    }
  }

  fn remove_runtime_file(path: &Path, relative_path: &str) -> usize {
    log::info!(
      "Checking file: {} (exists: {})",
      relative_path,
      path.exists()
    );
    if !path.exists() {
      return 0;
    }

    match std::fs::remove_file(path) {
      Ok(_) => {
        log::info!("✓ Removed: {}", relative_path);
        1
      }
      Err(e) => {
        log::error!("✗ Failed to remove {}: {}", relative_path, e);
        0
      }
    }
  }

  fn remove_runtime_dir(path: &Path, relative_path: &str) -> usize {
    log::info!(
      "Checking dir: {} (exists: {})",
      relative_path,
      path.exists()
    );
    if !path.exists() {
      return 0;
    }

    match std::fs::remove_dir_all(path) {
      Ok(_) => {
        log::info!("✓ Removed dir: {}", relative_path);
        1
      }
      Err(e) => {
        log::error!("✗ Failed to remove dir {}: {}", relative_path, e);
        0
      }
    }
  }

  fn clean_runtime_state_root(root_path: &Path, label: &str) -> usize {
    log::info!(
      "Cleaning runtime root [{}]: {}\nExists: {}",
      label,
      root_path.display(),
      root_path.exists()
    );

    if !root_path.exists() {
      return 0;
    }

    let files_to_remove = [
      "Local State",
      "Preferences",
      "Secure Preferences",
      "SingletonLock",
      "SingletonSocket",
      "SingletonCookie",
      "lockfile",
      "Default/Preferences",
      "Default/Secure Preferences",
      "Default/Current Session",
      "Default/Current Tabs",
      "Default/Last Session",
      "Default/Last Tabs",
      "Default/LOCK",
      "Default/LOG",
      "Default/LOG.old",
    ];
    let dirs_to_remove = [
      "Default/Sessions",
      "Default/Session Storage",
      "Default/Cache",
      "Default/Code Cache",
      "Default/GPUCache",
      "Default/DawnGraphiteCache",
      "Default/DawnWebGPUCache",
      "Default/Network",
      "Default/Service Worker/ScriptCache",
    ];

    let mut removed_count = 0;
    for relative_path in files_to_remove {
      removed_count += Self::remove_runtime_file(&root_path.join(relative_path), relative_path);
    }

    for relative_path in dirs_to_remove {
      removed_count += Self::remove_runtime_dir(&root_path.join(relative_path), relative_path);
    }

    removed_count
  }

  fn get_cloakbrowser_remap_profile_data_path(&self, profile_id: &uuid::Uuid) -> PathBuf {
    self
      .profile_manager
      .get_profiles_dir()
      .join(profile_id.to_string())
      .join("cloakbrowser-profile")
  }

  fn get_orbita_source_profile_data_path(&self, profile: &BrowserProfile) -> PathBuf {
    let profiles_dir = self.profile_manager.get_profiles_dir();
    profile.get_profile_data_path(&profiles_dir)
  }

  fn get_orbita_remap_migration_marker_path(&self, profile_id: &uuid::Uuid) -> PathBuf {
    self
      .get_cloakbrowser_remap_profile_data_path(profile_id)
      .join(".orbita-history-migrated")
  }

  fn copy_if_exists(
    source: &Path,
    destination: &Path,
    label: &str,
  ) -> Result<bool, Box<dyn std::error::Error>> {
    if !source.exists() {
      return Ok(false);
    }

    if let Some(parent) = destination.parent() {
      std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(source, destination)?;
    log::info!(
      "Migrated Orbita data file '{}' from {} to {}",
      label,
      source.display(),
      destination.display()
    );
    Ok(true)
  }

  fn migrate_orbita_profile_data_for_cloakbrowser(
    &self,
    profile: &BrowserProfile,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let source_root = self.get_orbita_source_profile_data_path(profile);
    let destination_root = self.get_cloakbrowser_remap_profile_data_path(&profile.id);
    let marker_path = self.get_orbita_remap_migration_marker_path(&profile.id);

    if marker_path.exists() {
      log::info!(
        "Orbita migration marker exists for profile '{}', skipping history migration",
        profile.name
      );
      return Ok(());
    }

    if !source_root.exists() {
      log::warn!(
        "Orbita source profile data missing for '{}', skipping migration: {}",
        profile.name,
        source_root.display()
      );
      return Ok(());
    }

    std::fs::create_dir_all(&destination_root)?;

    let files_to_copy = [
      "First Run",
      "Default/Bookmarks",
      "Default/Bookmarks.bak",
      "Default/History",
      "Default/History-journal",
      "Default/Favicons",
      "Default/Favicons-journal",
      "Default/Visited Links",
      "Default/Top Sites",
      "Default/Top Sites-journal",
      "Default/Shortcuts",
      "Default/Shortcuts-journal",
    ];

    let mut copied_count = 0;
    for relative_path in files_to_copy {
      let source_path = source_root.join(relative_path);
      let destination_path = destination_root.join(relative_path);
      if Self::copy_if_exists(&source_path, &destination_path, relative_path)? {
        copied_count += 1;
      }
    }

    std::fs::write(
      &marker_path,
      format!(
        "migrated_at={}
source={}
copied_files={}
",
        chrono::Utc::now().to_rfc3339(),
        source_root.display(),
        copied_count
      ),
    )?;

    log::info!(
      "Orbita history migration complete for '{}': copied {} files into {}",
      profile.name,
      copied_count,
      destination_root.display()
    );

    Ok(())
  }

  fn get_launch_profile_data_path(
    &self,
    profile: &BrowserProfile,
    orbita_remap_requested: bool,
  ) -> PathBuf {
    if orbita_remap_requested {
      return self.get_cloakbrowser_remap_profile_data_path(&profile.id);
    }

    let profiles_dir = self.profile_manager.get_profiles_dir();
    profile.get_profile_data_path(&profiles_dir)
  }

  fn clean_orbita_profile_data(
    &self,
    profile_data_path: &Path,
  ) -> Result<(), Box<dyn std::error::Error>> {
    log::info!(
      "=== CLEANING ORBITA PROFILE DATA ===\nPath: {}\nExists: {}",
      profile_data_path.display(),
      profile_data_path.exists()
    );

    if !profile_data_path.exists() {
      log::warn!("Profile data path does not exist, skipping clean");
      return Ok(());
    }

    let mut removed_count = Self::clean_runtime_state_root(profile_data_path, "profile-data");

    let app_data_root = self.base_dirs.data_local_dir().join(Self::app_dir_name());
    if let Ok(relative_path) = profile_data_path.strip_prefix(&app_data_root) {
      let cache_root = self
        .base_dirs
        .cache_dir()
        .join(Self::app_dir_name())
        .join(relative_path);
      removed_count += Self::clean_runtime_state_root(&cache_root, "cache-data");
    } else {
      log::warn!(
        "Profile data path is outside app data root, skipping cache cleanup: {}",
        profile_data_path.display()
      );
    }

    // Remove macOS Saved Application State directory (causes crash on restore)
    let saved_state_dir = profile_data_path.join("Saved Application State");
    log::info!(
      "Checking Saved Application State dir (exists: {})",
      saved_state_dir.exists()
    );
    if saved_state_dir.exists() {
      match std::fs::remove_dir_all(&saved_state_dir) {
        Ok(_) => {
          removed_count += 1;
          log::info!("✓ Removed: Saved Application State directory");
        }
        Err(e) => {
          log::error!("✗ Failed to remove Saved Application State: {}", e);
        }
      }
    }

    log::info!("=== CLEAN COMPLETE: {} items removed ===", removed_count);
    Ok(())
  }

  fn build_cloakbrowser_config(profile: &BrowserProfile) -> CloakBrowserConfig {
    let mut config = profile.cloakbrowser_config.clone().unwrap_or_default();

    if config.seed.is_none() {
      config.seed = Some(CloakBrowserConfig::generate_seed());
    }

    let browser_config = profile
      .orbita_config
      .as_ref()
      .or(profile.wayfern_config.as_ref());
    let fingerprint = browser_config
      .and_then(|cfg| cfg.fingerprint.as_ref())
      .and_then(|raw| serde_json::from_str::<Value>(raw).ok());

    if config.platform.is_none() {
      config.platform = browser_config
        .and_then(|cfg| cfg.os.as_deref())
        .and_then(Self::normalize_cloak_platform)
        .or_else(|| {
          fingerprint
            .as_ref()
            .and_then(|value| Self::json_string(value, "platform"))
            .and_then(|value| Self::normalize_cloak_platform(&value))
        })
        .or_else(|| {
          fingerprint
            .as_ref()
            .and_then(|value| Self::json_string(value, "navigator.platform"))
            .and_then(|value| Self::normalize_cloak_platform(&value))
        });
    }

    if config.timezone.is_none() {
      config.timezone = fingerprint
        .as_ref()
        .and_then(|value| Self::json_string(value, "timezone"));
    }

    if config.locale.is_none() {
      config.locale = fingerprint.as_ref().and_then(Self::locale_from_fingerprint);
    }

    if config.user_agent.is_none() {
      config.user_agent = fingerprint
        .as_ref()
        .and_then(|value| {
          Self::json_string(value, "userAgent")
            .or_else(|| Self::json_string(value, "navigator.userAgent"))
            .or_else(|| Self::json_string(value, "headers.User-Agent"))
        })
        .or_else(|| profile.user_agent.clone());
    }

    if config.platform.is_none() {
      config.platform = config
        .user_agent
        .as_deref()
        .and_then(Self::platform_from_user_agent);
    }

    config
  }

  #[cfg(target_os = "macos")]
  fn build_browser_launch_log_path(&self, profile: &BrowserProfile) -> PathBuf {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|duration| duration.as_secs())
      .unwrap_or_default();
    std::env::temp_dir().join(format!("foxia-browser-{}-{timestamp}.log", profile.id))
  }

  #[cfg(target_os = "macos")]
  fn latest_chromium_crash_report_path(&self) -> Option<PathBuf> {
    let reports_dir = self
      .base_dirs
      .home_dir()
      .join("Library")
      .join("Logs")
      .join("DiagnosticReports");

    let mut entries: Vec<_> = std::fs::read_dir(reports_dir)
      .ok()?
      .filter_map(Result::ok)
      .filter(|entry| {
        entry
          .file_name()
          .to_str()
          .map(|name| name.starts_with("Chromium-") && name.ends_with(".ips"))
          .unwrap_or(false)
      })
      .collect();

    entries.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    entries.last().map(|entry| entry.path())
  }

  #[cfg(target_os = "macos")]
  fn is_process_running_healthy(system: &System, pid: u32) -> bool {
    system
      .process(sysinfo::Pid::from(pid as usize))
      .map(|process| {
        !matches!(
          process.status(),
          ProcessStatus::Zombie | ProcessStatus::Dead
        )
      })
      .unwrap_or(false)
  }

  pub async fn launch_browser(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
    url: Option<String>,
    local_proxy_settings: Option<&ProxySettings>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error + Send + Sync>> {
    self
      .launch_browser_internal(app_handle, profile, url, local_proxy_settings, None, false)
      .await
  }

  async fn launch_browser_internal(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
    url: Option<String>,
    local_proxy_settings: Option<&ProxySettings>,
    remote_debugging_port: Option<u16>,
    headless: bool,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error + Send + Sync>> {
    let profile_to_persist = profile.clone();
    let orbita_remap_requested = profile.browser == "orbita";

    // Remap Orbita profiles to use CloakBrowser (Foxia Browser)
    let profile_owned = if profile.browser == "orbita" {
      log::info!(
        "Remapping Orbita profile '{}' to use CloakBrowser (Foxia Browser)",
        profile.name
      );
      let registry = crate::downloaded_browsers_registry::DownloadedBrowsersRegistry::instance();
      let mut cloakbrowser_versions = registry.get_downloaded_versions("cloakbrowser");
      cloakbrowser_versions.sort_by(|a, b| crate::api_client::compare_versions(b, a));

      let cloak_version = cloakbrowser_versions.into_iter().next().ok_or_else(|| {
        "No Foxia Browser version downloaded. Please download Foxia Browser first.".to_string()
      })?;

      if let Err(e) = self.migrate_orbita_profile_data_for_cloakbrowser(profile) {
        log::error!(
          "Failed to migrate Orbita history for '{}': {}",
          profile.name,
          e
        );
      }

      let cloak_config = Self::build_cloakbrowser_config(profile);

      // Clean Orbita-specific files from profile data dir to prevent conflicts
      let profile_data_path =
        self.get_launch_profile_data_path(profile, profile.browser == "orbita");
      log::info!(
        "Profile data path for Orbita->CloakBrowser remap: {}",
        profile_data_path.display()
      );
      if let Err(e) = self.clean_orbita_profile_data(&profile_data_path) {
        log::error!(
          "Failed to clean Orbita profile data for '{}': {}",
          profile.name,
          e
        );
      } else {
        log::info!(
          "Successfully cleaned Orbita profile data for '{}'",
          profile.name
        );
      }

      let mut remapped = profile.clone();
      remapped.browser = "cloakbrowser".to_string();
      remapped.version = cloak_version;
      remapped.cloakbrowser_config = Some(cloak_config);
      remapped
    } else {
      profile.clone()
    };
    let profile = &profile_owned;

    // Clean Orbita-specific files for CloakBrowser profiles on every launch
    // (handles profiles already remapped from Orbita in previous sessions)
    if profile.browser == "cloakbrowser" {
      let profile_data_path = self.get_launch_profile_data_path(profile, orbita_remap_requested);
      log::info!(
        "Cleaning profile data for CloakBrowser launch: {}",
        profile_data_path.display()
      );
      if let Err(e) = self.clean_orbita_profile_data(&profile_data_path) {
        log::error!("Failed to clean profile data for '{}': {}", profile.name, e);
      }

      #[cfg(target_os = "macos")]
      {
        let saved_state_path = self
          .base_dirs
          .home_dir()
          .join("Library")
          .join("Saved Application State")
          .join("org.chromium.Chromium.savedState");
        if saved_state_path.exists() {
          match std::fs::remove_dir_all(&saved_state_path) {
            Ok(_) => log::info!("Removed macOS savedState: {}", saved_state_path.display()),
            Err(e) => log::error!("Failed to remove macOS savedState: {}", e),
          }
        } else {
          log::info!(
            "macOS savedState not found (already clean): {}",
            saved_state_path.display()
          );
        }
      }
    }

    // Check if browser is disabled due to ongoing update
    if self.auto_updater.is_browser_disabled(&profile.browser)? {
      return Err(
        format!(
          "{} is currently being updated. Please wait for the update to complete.",
          profile.browser
        )
        .into(),
      );
    }

    // Handle Camoufox profiles using CamoufoxManager
    if profile.browser == "camoufox" {
      // Get or create camoufox config
      let mut camoufox_config = profile.camoufox_config.clone().unwrap_or_else(|| {
        log::info!(
          "No camoufox config found for profile {}, using default",
          profile.name
        );
        CamoufoxConfig::default()
      });

      // Always start a local proxy for Camoufox (for traffic monitoring and geoip support)
      let upstream_proxy = self.resolve_upstream_proxy(profile);

      log::info!(
        "Starting local proxy for Camoufox profile: {} (upstream: {})",
        profile.name,
        upstream_proxy
          .as_ref()
          .map(|p| format!("{}:{}", p.host, p.port))
          .unwrap_or_else(|| "DIRECT".to_string())
      );

      // Start the proxy and get local proxy settings
      // If proxy startup fails, DO NOT launch Camoufox - it requires local proxy
      let profile_id_str = profile.id.to_string();
      let local_proxy = PROXY_MANAGER
        .start_proxy(
          app_handle.clone(),
          upstream_proxy.as_ref(),
          0, // Use 0 as temporary PID, will be updated later
          Some(&profile_id_str),
        )
        .await
        .map_err(|e| {
          let error_msg = format!("Failed to start local proxy for Camoufox: {e}");
          log::error!("{}", error_msg);
          error_msg
        })?;

      // Format proxy URL for camoufox - always use HTTP for the local proxy
      let proxy_url = format!("http://{}:{}", local_proxy.host, local_proxy.port);

      // Set proxy in camoufox config
      camoufox_config.proxy = Some(proxy_url);

      // Ensure geoip is always enabled for proper geolocation spoofing
      if camoufox_config.geoip.is_none() {
        camoufox_config.geoip = Some(serde_json::Value::Bool(true));
      }

      log::info!(
        "Configured local proxy for Camoufox: {:?}, geoip: {:?}",
        camoufox_config.proxy,
        camoufox_config.geoip
      );

      // Check if we need to generate a new fingerprint on every launch
      let mut updated_profile = profile.clone();
      if camoufox_config.randomize_fingerprint_on_launch == Some(true) {
        log::info!(
          "Generating random fingerprint for Camoufox profile: {}",
          profile.name
        );

        // Create a config copy without the existing fingerprint to force generation of a new one
        let mut config_for_generation = camoufox_config.clone();
        config_for_generation.fingerprint = None;

        // Generate a new fingerprint
        let new_fingerprint = self
          .camoufox_manager
          .generate_fingerprint_config(&app_handle, profile, &config_for_generation)
          .await
          .map_err(|e| format!("Failed to generate random fingerprint: {e}"))?;

        log::info!(
          "New fingerprint generated, length: {} chars",
          new_fingerprint.len()
        );

        // Update the config with the new fingerprint for launching
        camoufox_config.fingerprint = Some(new_fingerprint.clone());

        // Save the updated fingerprint to the profile so it persists
        // We need to preserve all existing config fields and only update the fingerprint
        let mut updated_camoufox_config =
          updated_profile.camoufox_config.clone().unwrap_or_default();
        updated_camoufox_config.fingerprint = Some(new_fingerprint);
        // Preserve the randomize flag so it persists across launches
        updated_camoufox_config.randomize_fingerprint_on_launch = Some(true);
        // Preserve the OS setting so it's used for future fingerprint generation
        if camoufox_config.os.is_some() {
          updated_camoufox_config.os = camoufox_config.os.clone();
        }
        updated_profile.camoufox_config = Some(updated_camoufox_config.clone());

        log::info!(
          "Updated profile camoufox_config with new fingerprint for profile: {}, fingerprint length: {}",
          profile.name,
          updated_camoufox_config.fingerprint.as_ref().map(|f| f.len()).unwrap_or(0)
        );
      }

      // Launch Camoufox browser
      log::info!("Launching Camoufox for profile: {}", profile.name);
      let camoufox_result = self
        .camoufox_manager
        .launch_camoufox_profile(
          app_handle.clone(),
          updated_profile.clone(),
          camoufox_config,
          url,
        )
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
          format!("Failed to launch Camoufox: {e}").into()
        })?;

      // For server-based Camoufox, we use the process_id
      let process_id = camoufox_result.processId.unwrap_or(0);
      log::info!("Camoufox launched successfully with PID: {process_id}");

      // Update profile with the process info from camoufox result
      updated_profile.process_id = Some(process_id);
      updated_profile.last_launch = Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());

      // Update the proxy manager with the correct PID
      if let Err(e) = PROXY_MANAGER.update_proxy_pid(0, process_id) {
        log::warn!("Warning: Failed to update proxy PID mapping: {e}");
      } else {
        log::info!("Updated proxy PID mapping from temp (0) to actual PID: {process_id}");
      }

      // Save the updated profile (includes new fingerprint if randomize is enabled)
      log::info!(
        "Saving profile {} with camoufox_config fingerprint length: {}",
        updated_profile.name,
        updated_profile
          .camoufox_config
          .as_ref()
          .and_then(|c| c.fingerprint.as_ref())
          .map(|f| f.len())
          .unwrap_or(0)
      );
      self.save_process_info(&updated_profile)?;
      // Ensure tag suggestions include any tags from this profile
      let _ = crate::tag_manager::TAG_MANAGER.lock().map(|tm| {
        let _ = tm.rebuild_from_profiles(&self.profile_manager.list_profiles().unwrap_or_default());
      });
      log::info!(
        "Successfully saved profile with process info: {}",
        updated_profile.name
      );

      // Emit profiles-changed to trigger frontend to reload profiles from disk
      // This ensures the UI displays the newly generated fingerprint
      if let Err(e) = events::emit_empty("profiles-changed") {
        log::warn!("Warning: Failed to emit profiles-changed event: {e}");
      }

      log::info!(
        "Emitting profile events for successful Camoufox launch: {}",
        updated_profile.name
      );

      // Emit profile update event to frontend
      if let Err(e) = events::emit("profile-updated", &updated_profile) {
        log::warn!("Warning: Failed to emit profile update event: {e}");
      }

      // Emit minimal running changed event to frontend with a small delay
      #[derive(Serialize)]
      struct RunningChangedPayload {
        id: String,
        is_running: bool,
      }

      let payload = RunningChangedPayload {
        id: updated_profile.id.to_string(),
        is_running: updated_profile.process_id.is_some(),
      };

      if let Err(e) = events::emit("profile-running-changed", &payload) {
        log::warn!("Warning: Failed to emit profile running changed event: {e}");
      } else {
        log::info!(
          "Successfully emitted profile-running-changed event for Camoufox {}: running={}",
          updated_profile.name,
          payload.is_running
        );
      }

      return Ok(updated_profile);
    }

    // Handle Wayfern profiles using WayfernManager
    if profile.browser == "wayfern" {
      // Get or create wayfern config
      let mut wayfern_config = profile.wayfern_config.clone().unwrap_or_else(|| {
        log::info!(
          "No wayfern config found for profile {}, using default",
          profile.name
        );
        WayfernConfig::default()
      });

      // Always start a local proxy for Wayfern (for traffic monitoring and geoip support)
      let upstream_proxy = self.resolve_upstream_proxy(profile);

      log::info!(
        "Starting local proxy for Wayfern profile: {} (upstream: {})",
        profile.name,
        upstream_proxy
          .as_ref()
          .map(|p| format!("{}:{}", p.host, p.port))
          .unwrap_or_else(|| "DIRECT".to_string())
      );

      // Start the proxy and get local proxy settings
      // If proxy startup fails, DO NOT launch Wayfern - it requires local proxy
      let profile_id_str = profile.id.to_string();
      let local_proxy = PROXY_MANAGER
        .start_proxy(
          app_handle.clone(),
          upstream_proxy.as_ref(),
          0, // Use 0 as temporary PID, will be updated later
          Some(&profile_id_str),
        )
        .await
        .map_err(|e| {
          let error_msg = format!("Failed to start local proxy for Wayfern: {e}");
          log::error!("{}", error_msg);
          error_msg
        })?;

      // Format proxy URL for wayfern - always use HTTP for the local proxy
      let proxy_url = format!("http://{}:{}", local_proxy.host, local_proxy.port);

      // Set proxy in wayfern config
      wayfern_config.proxy = Some(proxy_url);

      log::info!(
        "Configured local proxy for Wayfern: {:?}",
        wayfern_config.proxy
      );

      // Check if we need to generate a new fingerprint on every launch
      let mut updated_profile = profile.clone();
      if wayfern_config.randomize_fingerprint_on_launch == Some(true) {
        log::info!(
          "Generating random fingerprint for Wayfern profile: {}",
          profile.name
        );

        // Create a config copy without the existing fingerprint to force generation of a new one
        let mut config_for_generation = wayfern_config.clone();
        config_for_generation.fingerprint = None;

        // Generate a new fingerprint
        let new_fingerprint = self
          .wayfern_manager
          .generate_fingerprint_config(&app_handle, profile, &config_for_generation)
          .await
          .map_err(|e| format!("Failed to generate random fingerprint: {e}"))?;

        log::info!(
          "New fingerprint generated, length: {} chars",
          new_fingerprint.len()
        );

        // Update the config with the new fingerprint for launching
        wayfern_config.fingerprint = Some(new_fingerprint.clone());

        // Save the updated fingerprint to the profile so it persists
        let mut updated_wayfern_config = updated_profile.wayfern_config.clone().unwrap_or_default();
        updated_wayfern_config.fingerprint = Some(new_fingerprint);
        updated_wayfern_config.randomize_fingerprint_on_launch = Some(true);
        if wayfern_config.os.is_some() {
          updated_wayfern_config.os = wayfern_config.os.clone();
        }
        updated_profile.wayfern_config = Some(updated_wayfern_config.clone());

        log::info!(
          "Updated profile wayfern_config with new fingerprint for profile: {}, fingerprint length: {}",
          profile.name,
          updated_wayfern_config.fingerprint.as_ref().map(|f| f.len()).unwrap_or(0)
        );
      }

      // Launch Wayfern browser
      log::info!("Launching Wayfern for profile: {}", profile.name);

      // Get profile path for Wayfern
      let profiles_dir = self.profile_manager.get_profiles_dir();
      let profile_data_path = updated_profile.get_profile_data_path(&profiles_dir);
      let profile_path_str = profile_data_path.to_string_lossy().to_string();

      // Get proxy URL from config
      let proxy_url = wayfern_config.proxy.as_deref();

      let wayfern_result = self
        .wayfern_manager
        .launch_wayfern(
          &app_handle,
          &updated_profile,
          &profile_path_str,
          &wayfern_config,
          url.as_deref(),
          proxy_url,
        )
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
          format!("Failed to launch Wayfern: {e}").into()
        })?;

      // Get the process ID from launch result
      let process_id = wayfern_result.processId.unwrap_or(0);
      log::info!("Wayfern launched successfully with PID: {process_id}");

      // Update profile with the process info
      updated_profile.process_id = Some(process_id);
      updated_profile.last_launch = Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());

      // Update the proxy manager with the correct PID
      if let Err(e) = PROXY_MANAGER.update_proxy_pid(0, process_id) {
        log::warn!("Warning: Failed to update proxy PID mapping: {e}");
      } else {
        log::info!("Updated proxy PID mapping from temp (0) to actual PID: {process_id}");
      }

      // Save the updated profile
      log::info!(
        "Saving profile {} with wayfern_config fingerprint length: {}",
        updated_profile.name,
        updated_profile
          .wayfern_config
          .as_ref()
          .and_then(|c| c.fingerprint.as_ref())
          .map(|f| f.len())
          .unwrap_or(0)
      );
      self.save_process_info(&updated_profile)?;
      let _ = crate::tag_manager::TAG_MANAGER.lock().map(|tm| {
        let _ = tm.rebuild_from_profiles(&self.profile_manager.list_profiles().unwrap_or_default());
      });
      log::info!(
        "Successfully saved profile with process info: {}",
        updated_profile.name
      );

      // Emit profiles-changed to trigger frontend to reload profiles from disk
      if let Err(e) = events::emit_empty("profiles-changed") {
        log::warn!("Warning: Failed to emit profiles-changed event: {e}");
      }

      log::info!(
        "Emitting profile events for successful Wayfern launch: {}",
        updated_profile.name
      );

      // Emit profile update event to frontend
      if let Err(e) = events::emit("profile-updated", &updated_profile) {
        log::warn!("Warning: Failed to emit profile update event: {e}");
      }

      // Emit minimal running changed event to frontend
      #[derive(Serialize)]
      struct RunningChangedPayload {
        id: String,
        is_running: bool,
      }

      let payload = RunningChangedPayload {
        id: updated_profile.id.to_string(),
        is_running: updated_profile.process_id.is_some(),
      };

      if let Err(e) = events::emit("profile-running-changed", &payload) {
        log::warn!("Warning: Failed to emit profile running changed event: {e}");
      } else {
        log::info!(
          "Successfully emitted profile-running-changed event for Wayfern {}: running={}",
          updated_profile.name,
          payload.is_running
        );
      }

      return Ok(updated_profile);
    }

    let url = if orbita_remap_requested && url.is_none() {
      log::info!(
        "Orbita remap launch without URL, forcing https://www.google.com to avoid Chromium internal page crash path"
      );
      Some("https://www.google.com".to_string())
    } else {
      url
    };

    // Create browser instance
    let browser_type = BrowserType::from_str(&profile.browser)
      .map_err(|_| format!("Invalid browser type: {}", profile.browser))?;
    let browser = create_browser(browser_type.clone());

    // Get executable path using common helper
    let executable_path = self.get_browser_executable_path(profile).map_err(|e| {
      let err_msg = format!("Failed to get executable path: {e}");
      log::error!("{}", err_msg);
      err_msg
    })?;

    log::info!("Executable path: {executable_path:?}");

    // Prepare the executable (set permissions, etc.)
    if let Err(e) = browser.prepare_executable(&executable_path) {
      log::warn!("Warning: Failed to prepare executable: {e}");
      // Continue anyway, the error might not be critical
    }

    // Get stored proxy settings for later use (removed as we handle this in proxy startup)
    let _stored_proxy_settings = profile
      .proxy_id
      .as_ref()
      .and_then(|id| PROXY_MANAGER.get_proxy_settings_by_id(id));

    // Use provided local proxy for Chromium-based browsers launch arguments
    let proxy_for_launch_args: Option<&ProxySettings> = local_proxy_settings;

    // Get profile data path and launch arguments
    let profile_data_path = self.get_launch_profile_data_path(profile, orbita_remap_requested);
    let browser_args = browser
      .create_launch_args(
        &profile_data_path.to_string_lossy(),
        proxy_for_launch_args,
        url,
        remote_debugging_port,
        headless,
      )
      .expect("Failed to create launch arguments");

    // For CloakBrowser, inject fingerprint args from config
    let resolved_cloakbrowser_config = if profile.browser == "cloakbrowser" {
      Some(Self::build_cloakbrowser_config(profile))
    } else {
      None
    };
    let browser_args = if profile.browser == "cloakbrowser" {
      let mut args = browser_args;
      let config = resolved_cloakbrowser_config
        .clone()
        .unwrap_or_else(CloakBrowserConfig::default);
      args.extend(config.to_launch_args());
      #[cfg(target_os = "macos")]
      {
        args.push("--enable-logging=stderr".to_string());
        args.push("--v=1".to_string());
      }
      args
    } else {
      browser_args
    };

    #[cfg(target_os = "macos")]
    let browser_log_path = if profile.browser == "cloakbrowser" {
      let path = self.build_browser_launch_log_path(profile);
      log::info!("CloakBrowser launch log path: {}", path.display());
      Some(path)
    } else {
      None
    };

    // Launch browser using platform-specific method
    let child = {
      #[cfg(target_os = "macos")]
      {
        let use_launch_services = profile.browser != "cloakbrowser";
        platform_browser::macos::launch_browser_process(
          &executable_path,
          &browser_args,
          use_launch_services,
          browser_log_path.as_deref(),
        )
        .await?
      }

      #[cfg(target_os = "windows")]
      {
        platform_browser::windows::launch_browser_process(&executable_path, &browser_args).await?
      }

      #[cfg(target_os = "linux")]
      {
        platform_browser::linux::launch_browser_process(&executable_path, &browser_args).await?
      }

      #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
      {
        return Err("Unsupported platform for browser launching".into());
      }
    };

    let launcher_pid = child.id();

    log::info!(
      "Launched browser with launcher PID: {} for profile: {} (ID: {})",
      launcher_pid,
      profile.name,
      profile.id
    );

    // On macOS, when launching via `open -a`, the child PID is the `open` helper.
    // Resolve and store the actual browser PID for all browser types.
    let actual_pid = {
      #[cfg(target_os = "macos")]
      {
        // Give the browser a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let system = System::new_all();
        let profile_data_path = self.get_launch_profile_data_path(profile, orbita_remap_requested);
        let profile_data_path_str = profile_data_path.to_string_lossy();

        let mut resolved_pid = launcher_pid;

        for (pid, process) in system.processes() {
          let cmd: Vec<String> = process
            .cmd()
            .iter()
            .filter_map(|arg| arg.to_str().map(str::to_owned))
            .collect();
          if cmd.is_empty() {
            continue;
          }

          let exe_name = process.name().to_string_lossy().to_string();
          if !Self::process_matches_browser(&profile.browser, &exe_name, &cmd) {
            continue;
          }

          if Self::process_matches_profile_path(&profile.browser, &cmd, &profile_data_path_str) {
            let pid_u32 = pid.as_u32();
            if pid_u32 != launcher_pid {
              resolved_pid = pid_u32;
              break;
            }
          }
        }

        if profile.browser == "cloakbrowser"
          && !Self::is_process_running_healthy(&system, resolved_pid)
        {
          let latest_crash_report = self
            .latest_chromium_crash_report_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not found".to_string());
          let browser_log_path = browser_log_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not configured".to_string());

          return Err(
            format!(
              "CloakBrowser exited during launch. latest crash report: {latest_crash_report}. browser log: {browser_log_path}"
            )
            .into(),
          );
        }

        resolved_pid
      }

      #[cfg(not(target_os = "macos"))]
      {
        launcher_pid
      }
    };

    // Update profile with process info and save
    let mut updated_profile = profile_to_persist;
    updated_profile.process_id = Some(actual_pid);
    updated_profile.last_launch = Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    if let Some(config) = resolved_cloakbrowser_config {
      updated_profile.cloakbrowser_config = Some(config);
    }

    self.save_process_info(&updated_profile)?;
    let _ = crate::tag_manager::TAG_MANAGER.lock().map(|tm| {
      let _ = tm.rebuild_from_profiles(&self.profile_manager.list_profiles().unwrap_or_default());
    });

    // Apply proxy settings if needed (for Firefox-based browsers)
    if profile.proxy_id.is_some()
      && matches!(
        browser_type,
        BrowserType::Firefox | BrowserType::FirefoxDeveloper | BrowserType::Zen
      )
    {
      // Proxy settings for Firefox-based browsers are applied via user.js file
      // which is already handled in the profile creation process
    }

    log::info!(
      "Emitting profile events for successful launch: {} (ID: {})",
      updated_profile.name,
      updated_profile.id
    );

    // Emit profile update event to frontend
    if let Err(e) = events::emit("profile-updated", &updated_profile) {
      log::warn!("Warning: Failed to emit profile update event: {e}");
    }

    // Emit minimal running changed event to frontend with a small delay to ensure UI consistency
    #[derive(Serialize)]
    struct RunningChangedPayload {
      id: String,
      is_running: bool,
    }
    let payload = RunningChangedPayload {
      id: updated_profile.id.to_string(),
      is_running: updated_profile.process_id.is_some(),
    };

    if let Err(e) = events::emit("profile-running-changed", &payload) {
      log::warn!("Warning: Failed to emit profile running changed event: {e}");
    } else {
      log::info!(
        "Successfully emitted profile-running-changed event for {}: running={}",
        updated_profile.name,
        payload.is_running
      );
    }

    Ok(updated_profile)
  }

  pub async fn open_url_in_existing_browser(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
    url: &str,
    _internal_proxy_settings: Option<&ProxySettings>,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Handle Camoufox profiles using CamoufoxManager
    if profile.browser == "camoufox" {
      // Get the profile path based on the UUID
      let profiles_dir = self.profile_manager.get_profiles_dir();
      let profile_data_path = profile.get_profile_data_path(&profiles_dir);
      let profile_path_str = profile_data_path.to_string_lossy();

      // Check if the process is running
      match self
        .camoufox_manager
        .find_camoufox_by_profile(&profile_path_str)
        .await
      {
        Ok(Some(_camoufox_process)) => {
          log::info!(
            "Opening URL in existing Camoufox process for profile: {} (ID: {})",
            profile.name,
            profile.id
          );

          // Get Camoufox executable path and use Firefox-like remote mechanism
          let executable_path = self
            .get_browser_executable_path(profile)
            .map_err(|e| format!("Failed to get Camoufox executable path: {e}"))?;

          // Launch Camoufox with -profile and -new-tab to open URL in existing instance
          // This works because we no longer use -no-remote flag
          let output = std::process::Command::new(&executable_path)
            .arg("-profile")
            .arg(&*profile_path_str)
            .arg("-new-tab")
            .arg(url)
            .output()
            .map_err(|e| format!("Failed to execute Camoufox: {e}"))?;

          if output.status.success() {
            log::info!("Successfully opened URL in existing Camoufox instance");
            return Ok(());
          } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Camoufox -new-tab command failed: {stderr}");
            return Err(
              format!("Failed to open URL in existing Camoufox instance: {stderr}").into(),
            );
          }
        }
        Ok(None) => {
          return Err("Camoufox browser is not running".into());
        }
        Err(e) => {
          return Err(format!("Error checking Camoufox process: {e}").into());
        }
      }
    }

    // Handle Wayfern profiles using WayfernManager
    if profile.browser == "wayfern" {
      let profiles_dir = self.profile_manager.get_profiles_dir();
      let profile_data_path = profile.get_profile_data_path(&profiles_dir);
      let profile_path_str = profile_data_path.to_string_lossy();

      // Check if the process is running
      match self
        .wayfern_manager
        .find_wayfern_by_profile(&profile_path_str)
        .await
      {
        Some(_wayfern_process) => {
          log::info!(
            "Opening URL in existing Wayfern process for profile: {} (ID: {})",
            profile.name,
            profile.id
          );

          // Use CDP to open URL in a new tab
          self
            .wayfern_manager
            .open_url_in_tab(&profile_path_str, url)
            .await?;
          return Ok(());
        }
        None => {
          return Err("Wayfern browser is not running".into());
        }
      }
    }

    // Use the comprehensive browser status check for non-camoufox/wayfern browsers
    let is_running = self
      .check_browser_status(app_handle.clone(), profile)
      .await?;

    if !is_running {
      return Err("Browser is not running".into());
    }

    // Get the updated profile with current PID
    let profiles = self
      .profile_manager
      .list_profiles()
      .expect("Failed to list profiles");
    let updated_profile = profiles
      .into_iter()
      .find(|p| p.id == profile.id)
      .unwrap_or_else(|| profile.clone());

    // Ensure we have a valid process ID
    if updated_profile.process_id.is_none() {
      return Err("No valid process ID found for the browser".into());
    }

    let browser_type = BrowserType::from_str(&updated_profile.browser)
      .map_err(|_| format!("Invalid browser type: {}", updated_profile.browser))?;

    // Get browser directory for all platforms - path structure: binaries/<browser>/<version>/
    let mut browser_dir = self.get_binaries_dir();
    browser_dir.push(&updated_profile.browser);
    browser_dir.push(&updated_profile.version);

    match browser_type {
      BrowserType::Firefox | BrowserType::FirefoxDeveloper | BrowserType::Zen => {
        #[cfg(target_os = "macos")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::macos::open_url_in_existing_browser_firefox_like(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(target_os = "windows")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::windows::open_url_in_existing_browser_firefox_like(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(target_os = "linux")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::linux::open_url_in_existing_browser_firefox_like(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Err("Unsupported platform".into());
      }
      BrowserType::Camoufox => {
        // Camoufox URL opening is handled differently
        Err("URL opening in existing Camoufox instance is not supported".into())
      }
      BrowserType::Wayfern => {
        // Wayfern URL opening is handled differently
        Err("URL opening in existing Wayfern instance is not supported".into())
      }
      BrowserType::Chromium
      | BrowserType::Brave
      | BrowserType::Orbita
      | BrowserType::CloakBrowser => {
        #[cfg(target_os = "macos")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::macos::open_url_in_existing_browser_chromium(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(target_os = "windows")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::windows::open_url_in_existing_browser_chromium(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(target_os = "linux")]
        {
          let profiles_dir = self.profile_manager.get_profiles_dir();
          return platform_browser::linux::open_url_in_existing_browser_chromium(
            &updated_profile,
            url,
            browser_type,
            &browser_dir,
            &profiles_dir,
          )
          .await;
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Err("Unsupported platform".into());
      }
    }
  }

  pub async fn launch_browser_with_debugging(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
    url: Option<String>,
    remote_debugging_port: Option<u16>,
    headless: bool,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error + Send + Sync>> {
    // Always start a local proxy for API launches
    // Determine upstream proxy if configured; otherwise use DIRECT
    let upstream_proxy = self.resolve_upstream_proxy(profile);

    // Use a temporary PID (1) to start the proxy, we'll update it after browser launch
    let temp_pid = 1u32;
    let profile_id_str = profile.id.to_string();

    // Start local proxy - if this fails, DO NOT launch browser
    let internal_proxy = PROXY_MANAGER
      .start_proxy(
        app_handle.clone(),
        upstream_proxy.as_ref(),
        temp_pid,
        Some(&profile_id_str),
      )
      .await
      .map_err(|e| {
        let error_msg = format!("Failed to start local proxy: {e}");
        log::error!("{}", error_msg);
        error_msg
      })?;

    let internal_proxy_settings = Some(internal_proxy.clone());

    // Configure Firefox profiles to use local proxy
    {
      // For Firefox-based browsers, apply PAC/user.js to point to the local proxy
      if matches!(
        profile.browser.as_str(),
        "firefox" | "firefox-developer" | "zen"
      ) {
        let profiles_dir = self.profile_manager.get_profiles_dir();
        let profile_path = profiles_dir.join(profile.id.to_string()).join("profile");

        // Provide a dummy upstream (ignored when internal proxy is provided)
        let dummy_upstream = ProxySettings {
          proxy_type: "http".to_string(),
          host: "127.0.0.1".to_string(),
          port: internal_proxy.port,
          username: None,
          password: None,
        };

        self
          .profile_manager
          .apply_proxy_settings_to_profile(&profile_path, &dummy_upstream, Some(&internal_proxy))
          .map_err(|e| format!("Failed to update profile proxy: {e}"))?;
      }
    }

    let result = self
      .launch_browser_internal(
        app_handle.clone(),
        profile,
        url,
        internal_proxy_settings.as_ref(),
        remote_debugging_port,
        headless,
      )
      .await;

    // Update proxy with correct PID if launch succeeded
    if let Ok(ref updated_profile) = result {
      if let Some(actual_pid) = updated_profile.process_id {
        let _ = PROXY_MANAGER.update_proxy_pid(temp_pid, actual_pid);
      }
    }

    result
  }

  pub async fn launch_or_open_url(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
    url: Option<String>,
    internal_proxy_settings: Option<&ProxySettings>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error + Send + Sync>> {
    log::info!(
      "launch_or_open_url called for profile: {} (ID: {})",
      profile.name,
      profile.id
    );

    // Get the most up-to-date profile data
    let profiles = self
      .profile_manager
      .list_profiles()
      .map_err(|e| format!("Failed to list profiles in launch_or_open_url: {e}"))?;
    let mut updated_profile = profiles
      .into_iter()
      .find(|p| p.id == profile.id)
      .unwrap_or_else(|| profile.clone());

    // Merge dynamic proxy from passed profile if present
    if profile.odoo_proxy.is_some() {
      updated_profile.odoo_proxy = profile.odoo_proxy.clone();
    }

    log::info!(
      "Checking browser status for profile: {} (ID: {})",
      updated_profile.name,
      updated_profile.id
    );

    // Check if browser is already running
    let is_running = self
      .check_browser_status(app_handle.clone(), &updated_profile)
      .await
      .map_err(|e| format!("Failed to check browser status: {e}"))?;

    // Get the updated profile again after status check (PID might have been updated)
    let profiles = self
      .profile_manager
      .list_profiles()
      .map_err(|e| format!("Failed to list profiles after status check: {e}"))?;
    let mut final_profile = profiles
      .into_iter()
      .find(|p| p.id == profile.id)
      .unwrap_or_else(|| updated_profile.clone());

    // Merge dynamic proxy again to ensure it carries through to launch
    if profile.odoo_proxy.is_some() {
      final_profile.odoo_proxy = profile.odoo_proxy.clone();
    }

    log::info!(
      "Browser status check - Profile: {} (ID: {}), Running: {}, URL: {:?}, PID: {:?}",
      final_profile.name,
      final_profile.id,
      is_running,
      url,
      final_profile.process_id
    );

    if is_running && url.is_some() {
      // Browser is running and we have a URL to open
      if let Some(url_ref) = url.as_ref() {
        log::info!("Opening URL in existing browser: {url_ref}");

        match self
          .open_url_in_existing_browser(
            app_handle.clone(),
            &final_profile,
            url_ref,
            internal_proxy_settings,
          )
          .await
        {
          Ok(()) => {
            log::info!("Successfully opened URL in existing browser");
            Ok(final_profile)
          }
          Err(e) => {
            log::info!("Failed to open URL in existing browser: {e}");

            // Fall back to launching a new instance
            log::info!(
              "Falling back to new instance for browser: {}",
              final_profile.browser
            );
            // Fallback to launching a new instance for other browsers
            self
              .launch_browser_internal(
                app_handle.clone(),
                &final_profile,
                url,
                internal_proxy_settings,
                None,
                false,
              )
              .await
          }
        }
      } else {
        // This case shouldn't happen since we checked is_some() above, but handle it gracefully
        log::info!("URL was unexpectedly None, launching new browser instance");
        self
          .launch_browser(
            app_handle.clone(),
            &final_profile,
            url,
            internal_proxy_settings,
          )
          .await
      }
    } else {
      // Browser is not running or no URL provided, launch new instance
      if !is_running {
        log::info!("Launching new browser instance - browser not running");
      } else {
        log::info!("Launching new browser instance - no URL provided");
      }
      self
        .launch_browser_internal(
          app_handle.clone(),
          &final_profile,
          url,
          internal_proxy_settings,
          None,
          false,
        )
        .await
    }
  }

  fn save_process_info(
    &self,
    profile: &BrowserProfile,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use the regular save_profile method which handles the UUID structure
    self.profile_manager.save_profile(profile).map_err(|e| {
      let error_string = e.to_string();
      Box::new(std::io::Error::other(error_string)) as Box<dyn std::error::Error + Send + Sync>
    })
  }

  pub async fn check_browser_status(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
  ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if profile.browser == "camoufox" {
      return self
        .profile_manager
        .check_browser_status(app_handle, profile)
        .await;
    }

    if let Some(process_id) = profile.process_id {
      let system = System::new_all();
      if system
        .process(sysinfo::Pid::from(process_id as usize))
        .is_some()
      {
        return Ok(true);
      }
    }

    let matching_pids = self.find_browser_processes_by_profile(profile);
    if !matching_pids.is_empty() {
      return Ok(true);
    }

    Ok(false)
  }

  pub async fn kill_browser_process(
    &self,
    app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Handle Camoufox profiles using CamoufoxManager
    if profile.browser == "camoufox" {
      // Search by profile path to find the running Camoufox instance
      let profiles_dir = self.profile_manager.get_profiles_dir();
      let profile_data_path = profile.get_profile_data_path(&profiles_dir);
      let profile_path_str = profile_data_path.to_string_lossy();

      log::info!(
        "Attempting to kill Camoufox process for profile: {} (ID: {})",
        profile.name,
        profile.id
      );

      // Stop the proxy associated with this profile first
      let profile_id_str = profile.id.to_string();
      if let Err(e) = PROXY_MANAGER
        .stop_proxy_by_profile_id(app_handle.clone(), &profile_id_str)
        .await
      {
        log::warn!(
          "Warning: Failed to stop proxy for profile {}: {e}",
          profile_id_str
        );
      }

      let mut process_actually_stopped = false;
      match self
        .camoufox_manager
        .find_camoufox_by_profile(&profile_path_str)
        .await
      {
        Ok(Some(camoufox_process)) => {
          log::info!(
            "Found Camoufox process: {} (PID: {:?})",
            camoufox_process.id,
            camoufox_process.processId
          );

          match self
            .camoufox_manager
            .stop_camoufox(&app_handle, &camoufox_process.id)
            .await
          {
            Ok(stopped) => {
              if let Some(pid) = camoufox_process.processId {
                if stopped {
                  // Verify the process actually died by checking after a short delay
                  use tokio::time::{sleep, Duration};
                  sleep(Duration::from_millis(500)).await;

                  use sysinfo::{Pid, System};
                  let system = System::new_all();
                  process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();

                  if process_actually_stopped {
                    log::info!(
                      "Successfully stopped Camoufox process: {} (PID: {:?}) - verified process is dead",
                      camoufox_process.id,
                      pid
                    );
                  } else {
                    log::warn!(
                      "Camoufox stop command returned success but process {} (PID: {:?}) is still running - forcing kill",
                      camoufox_process.id,
                      pid
                    );
                    // Force kill the process
                    #[cfg(target_os = "macos")]
                    {
                      use crate::platform_browser;
                      if let Err(e) = platform_browser::macos::kill_browser_process_impl(
                        pid,
                        Some(&profile_path_str),
                      )
                      .await
                      {
                        log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                      } else {
                        // Verify the process is actually dead after force kill
                        use tokio::time::{sleep, Duration};
                        sleep(Duration::from_millis(500)).await;
                        use sysinfo::{Pid, System};
                        let system = System::new_all();
                        process_actually_stopped =
                          system.process(Pid::from(pid as usize)).is_none();
                        if process_actually_stopped {
                          log::info!(
                            "Successfully force killed Camoufox process {} (PID: {:?})",
                            camoufox_process.id,
                            pid
                          );
                        }
                      }
                    }
                    #[cfg(target_os = "linux")]
                    {
                      use crate::platform_browser;
                      if let Err(e) = platform_browser::linux::kill_browser_process_impl(pid).await
                      {
                        log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                      } else {
                        // Verify the process is actually dead after force kill
                        use tokio::time::{sleep, Duration};
                        sleep(Duration::from_millis(500)).await;
                        use sysinfo::{Pid, System};
                        let system = System::new_all();
                        process_actually_stopped =
                          system.process(Pid::from(pid as usize)).is_none();
                        if process_actually_stopped {
                          log::info!(
                            "Successfully force killed Camoufox process {} (PID: {:?})",
                            camoufox_process.id,
                            pid
                          );
                        }
                      }
                    }
                    #[cfg(target_os = "windows")]
                    {
                      use crate::platform_browser;
                      if let Err(e) =
                        platform_browser::windows::kill_browser_process_impl(pid).await
                      {
                        log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                      } else {
                        // Verify the process is actually dead after force kill
                        use tokio::time::{sleep, Duration};
                        sleep(Duration::from_millis(500)).await;
                        use sysinfo::{Pid, System};
                        let system = System::new_all();
                        process_actually_stopped =
                          system.process(Pid::from(pid as usize)).is_none();
                        if process_actually_stopped {
                          log::info!(
                            "Successfully force killed Camoufox process {} (PID: {:?})",
                            camoufox_process.id,
                            pid
                          );
                        }
                      }
                    }
                  }
                } else {
                  // stop_camoufox returned false, try to force kill the process
                  log::warn!(
                    "Camoufox stop command returned false for process {} (PID: {:?}) - attempting force kill",
                    camoufox_process.id,
                    pid
                  );
                  #[cfg(target_os = "macos")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::macos::kill_browser_process_impl(
                      pid,
                      Some(&profile_path_str),
                    )
                    .await
                    {
                      log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                    } else {
                      // Verify the process is actually dead after force kill
                      use tokio::time::{sleep, Duration};
                      sleep(Duration::from_millis(500)).await;
                      use sysinfo::{Pid, System};
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Camoufox process {} (PID: {:?})",
                          camoufox_process.id,
                          pid
                        );
                      }
                    }
                  }
                  #[cfg(target_os = "linux")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::linux::kill_browser_process_impl(pid).await {
                      log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                    } else {
                      // Verify the process is actually dead after force kill
                      use tokio::time::{sleep, Duration};
                      sleep(Duration::from_millis(500)).await;
                      use sysinfo::{Pid, System};
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Camoufox process {} (PID: {:?})",
                          camoufox_process.id,
                          pid
                        );
                      }
                    }
                  }
                  #[cfg(target_os = "windows")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::windows::kill_browser_process_impl(pid).await
                    {
                      log::error!("Failed to force kill Camoufox process {}: {}", pid, e);
                    } else {
                      // Verify the process is actually dead after force kill
                      use tokio::time::{sleep, Duration};
                      sleep(Duration::from_millis(500)).await;
                      use sysinfo::{Pid, System};
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Camoufox process {} (PID: {:?})",
                          camoufox_process.id,
                          pid
                        );
                      }
                    }
                  }
                }
              } else {
                // No PID available, assume stopped if stop_camoufox returned true
                process_actually_stopped = stopped;
                if !stopped {
                  log::warn!(
                    "Failed to stop Camoufox process {} but no PID available for force kill",
                    camoufox_process.id
                  );
                }
              }
            }
            Err(e) => {
              log::error!(
                "Error stopping Camoufox process {}: {}",
                camoufox_process.id,
                e
              );
              // Try to force kill if we have a PID
              if let Some(pid) = camoufox_process.processId {
                log::info!(
                  "Attempting force kill after stop_camoufox error for PID: {}",
                  pid
                );
                #[cfg(target_os = "macos")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::macos::kill_browser_process_impl(pid, Some(&profile_path_str))
                      .await
                  {
                    log::error!(
                      "Failed to force kill Camoufox process {}: {}",
                      pid,
                      kill_err
                    );
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
                #[cfg(target_os = "linux")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::linux::kill_browser_process_impl(pid).await
                  {
                    log::error!(
                      "Failed to force kill Camoufox process {}: {}",
                      pid,
                      kill_err
                    );
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
                #[cfg(target_os = "windows")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::windows::kill_browser_process_impl(pid).await
                  {
                    log::error!(
                      "Failed to force kill Camoufox process {}: {}",
                      pid,
                      kill_err
                    );
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
              }
            }
          }
        }
        Ok(None) => {
          log::info!(
            "No running Camoufox process found for profile: {} (ID: {})",
            profile.name,
            profile.id
          );
          process_actually_stopped = true; // No process found, consider it stopped
        }
        Err(e) => {
          log::error!(
            "Error finding Camoufox process for profile {}: {}",
            profile.name,
            e
          );
        }
      }

      // If process wasn't confirmed stopped, return an error
      if !process_actually_stopped {
        log::error!(
          "Failed to stop Camoufox process for profile: {} (ID: {}) - process may still be running",
          profile.name,
          profile.id
        );
        return Err(
          format!(
            "Failed to stop Camoufox process for profile {} - process may still be running",
            profile.name
          )
          .into(),
        );
      }

      // Clear the process ID from the profile
      let mut updated_profile = profile.clone();
      updated_profile.process_id = None;

      // Check for pending updates and apply them for Camoufox profiles too
      if let Ok(Some(pending_update)) = self
        .auto_updater
        .get_pending_update(&profile.browser, &profile.version)
      {
        log::info!(
          "Found pending update for Camoufox profile {}: {} -> {}",
          profile.name,
          profile.version,
          pending_update.new_version
        );

        // Update the profile to the new version
        match self.profile_manager.update_profile_version(
          &app_handle,
          &profile.id.to_string(),
          &pending_update.new_version,
        ) {
          Ok(updated_profile_after_update) => {
            log::info!(
              "Successfully updated Camoufox profile {} from version {} to {}",
              profile.name,
              profile.version,
              pending_update.new_version
            );
            updated_profile = updated_profile_after_update;

            // Remove the pending update from the auto updater state
            if let Err(e) = self
              .auto_updater
              .dismiss_update_notification(&pending_update.id)
            {
              log::warn!("Warning: Failed to dismiss pending update notification: {e}");
            }
          }
          Err(e) => {
            log::error!(
              "Failed to apply pending update for Camoufox profile {}: {}",
              profile.name,
              e
            );
            // Continue with the original profile update (just clearing process_id)
          }
        }
      }

      self
        .save_process_info(&updated_profile)
        .map_err(|e| format!("Failed to update profile: {e}"))?;

      log::info!(
        "Emitting profile events for successful Camoufox kill: {}",
        updated_profile.name
      );

      // Emit profile update event to frontend
      if let Err(e) = events::emit("profile-updated", &updated_profile) {
        log::warn!("Warning: Failed to emit profile update event: {e}");
      }

      // Emit minimal running changed event to frontend immediately
      #[derive(Serialize)]
      struct RunningChangedPayload {
        id: String,
        is_running: bool,
      }
      let payload = RunningChangedPayload {
        id: updated_profile.id.to_string(),
        is_running: false, // Explicitly set to false since we just killed it
      };

      if let Err(e) = events::emit("profile-running-changed", &payload) {
        log::warn!("Warning: Failed to emit profile running changed event: {e}");
      } else {
        log::info!(
          "Successfully emitted profile-running-changed event for Camoufox {}: running={}",
          updated_profile.name,
          payload.is_running
        );
      }

      log::info!(
        "Camoufox process cleanup completed for profile: {} (ID: {})",
        profile.name,
        profile.id
      );

      // Consolidate browser versions after stopping a browser
      if let Ok(consolidated) = self
        .downloaded_browsers_registry
        .consolidate_browser_versions(&app_handle)
      {
        if !consolidated.is_empty() {
          log::info!("Post-stop version consolidation results:");
          for action in &consolidated {
            log::info!("  {action}");
          }
        }
      }

      return Ok(());
    }

    // Handle Wayfern profiles using WayfernManager
    if profile.browser == "wayfern" {
      let profiles_dir = self.profile_manager.get_profiles_dir();
      let profile_data_path = profile.get_profile_data_path(&profiles_dir);
      let profile_path_str = profile_data_path.to_string_lossy();

      log::info!(
        "Attempting to kill Wayfern process for profile: {} (ID: {})",
        profile.name,
        profile.id
      );

      // Stop the proxy associated with this profile first
      let profile_id_str = profile.id.to_string();
      if let Err(e) = PROXY_MANAGER
        .stop_proxy_by_profile_id(app_handle.clone(), &profile_id_str)
        .await
      {
        log::warn!(
          "Warning: Failed to stop proxy for profile {}: {e}",
          profile_id_str
        );
      }

      let mut process_actually_stopped = false;
      match self
        .wayfern_manager
        .find_wayfern_by_profile(&profile_path_str)
        .await
      {
        Some(wayfern_process) => {
          log::info!(
            "Found Wayfern process: {} (PID: {:?})",
            wayfern_process.id,
            wayfern_process.processId
          );

          match self.wayfern_manager.stop_wayfern(&wayfern_process.id).await {
            Ok(_) => {
              if let Some(pid) = wayfern_process.processId {
                // Verify the process actually died by checking after a short delay
                use tokio::time::{sleep, Duration};
                sleep(Duration::from_millis(500)).await;

                use sysinfo::{Pid, System};
                let system = System::new_all();
                process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();

                if process_actually_stopped {
                  log::info!(
                    "Successfully stopped Wayfern process: {} (PID: {:?}) - verified process is dead",
                    wayfern_process.id,
                    pid
                  );
                } else {
                  log::warn!(
                    "Wayfern stop command returned success but process {} (PID: {:?}) is still running - forcing kill",
                    wayfern_process.id,
                    pid
                  );
                  // Force kill the process
                  #[cfg(target_os = "macos")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::macos::kill_browser_process_impl(
                      pid,
                      Some(&profile_path_str),
                    )
                    .await
                    {
                      log::error!("Failed to force kill Wayfern process {}: {}", pid, e);
                    } else {
                      sleep(Duration::from_millis(500)).await;
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Wayfern process {} (PID: {:?})",
                          wayfern_process.id,
                          pid
                        );
                      }
                    }
                  }
                  #[cfg(target_os = "linux")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::linux::kill_browser_process_impl(pid).await {
                      log::error!("Failed to force kill Wayfern process {}: {}", pid, e);
                    } else {
                      sleep(Duration::from_millis(500)).await;
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Wayfern process {} (PID: {:?})",
                          wayfern_process.id,
                          pid
                        );
                      }
                    }
                  }
                  #[cfg(target_os = "windows")]
                  {
                    use crate::platform_browser;
                    if let Err(e) = platform_browser::windows::kill_browser_process_impl(pid).await
                    {
                      log::error!("Failed to force kill Wayfern process {}: {}", pid, e);
                    } else {
                      sleep(Duration::from_millis(500)).await;
                      let system = System::new_all();
                      process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                      if process_actually_stopped {
                        log::info!(
                          "Successfully force killed Wayfern process {} (PID: {:?})",
                          wayfern_process.id,
                          pid
                        );
                      }
                    }
                  }
                }
              } else {
                process_actually_stopped = true;
              }
            }
            Err(e) => {
              log::error!(
                "Error stopping Wayfern process {}: {}",
                wayfern_process.id,
                e
              );
              // Try to force kill if we have a PID
              if let Some(pid) = wayfern_process.processId {
                log::info!(
                  "Attempting force kill after stop_wayfern error for PID: {}",
                  pid
                );
                #[cfg(target_os = "macos")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::macos::kill_browser_process_impl(pid, Some(&profile_path_str))
                      .await
                  {
                    log::error!("Failed to force kill Wayfern process {}: {}", pid, kill_err);
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
                #[cfg(target_os = "linux")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::linux::kill_browser_process_impl(pid).await
                  {
                    log::error!("Failed to force kill Wayfern process {}: {}", pid, kill_err);
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
                #[cfg(target_os = "windows")]
                {
                  use crate::platform_browser;
                  if let Err(kill_err) =
                    platform_browser::windows::kill_browser_process_impl(pid).await
                  {
                    log::error!("Failed to force kill Wayfern process {}: {}", pid, kill_err);
                  } else {
                    use tokio::time::{sleep, Duration};
                    sleep(Duration::from_millis(500)).await;
                    use sysinfo::{Pid, System};
                    let system = System::new_all();
                    process_actually_stopped = system.process(Pid::from(pid as usize)).is_none();
                  }
                }
              }
            }
          }
        }
        None => {
          log::info!(
            "No running Wayfern process found for profile: {} (ID: {})",
            profile.name,
            profile.id
          );
          process_actually_stopped = true;
        }
      }

      // If process wasn't confirmed stopped, return an error
      if !process_actually_stopped {
        log::error!(
          "Failed to stop Wayfern process for profile: {} (ID: {}) - process may still be running",
          profile.name,
          profile.id
        );
        return Err(
          format!(
            "Failed to stop Wayfern process for profile {} - process may still be running",
            profile.name
          )
          .into(),
        );
      }

      // Clear the process ID from the profile
      let mut updated_profile = profile.clone();
      updated_profile.process_id = None;

      // Check for pending updates and apply them
      if let Ok(Some(pending_update)) = self
        .auto_updater
        .get_pending_update(&profile.browser, &profile.version)
      {
        log::info!(
          "Found pending update for Wayfern profile {}: {} -> {}",
          profile.name,
          profile.version,
          pending_update.new_version
        );

        match self.profile_manager.update_profile_version(
          &app_handle,
          &profile.id.to_string(),
          &pending_update.new_version,
        ) {
          Ok(updated_profile_after_update) => {
            log::info!(
              "Successfully updated Wayfern profile {} from version {} to {}",
              profile.name,
              profile.version,
              pending_update.new_version
            );
            updated_profile = updated_profile_after_update;

            if let Err(e) = self
              .auto_updater
              .dismiss_update_notification(&pending_update.id)
            {
              log::warn!("Warning: Failed to dismiss pending update notification: {e}");
            }
          }
          Err(e) => {
            log::error!(
              "Failed to apply pending update for Wayfern profile {}: {}",
              profile.name,
              e
            );
          }
        }
      }

      self
        .save_process_info(&updated_profile)
        .map_err(|e| format!("Failed to update profile: {e}"))?;

      log::info!(
        "Emitting profile events for successful Wayfern kill: {}",
        updated_profile.name
      );

      // Emit profile update event to frontend
      if let Err(e) = events::emit("profile-updated", &updated_profile) {
        log::warn!("Warning: Failed to emit profile update event: {e}");
      }

      // Emit minimal running changed event
      #[derive(Serialize)]
      struct RunningChangedPayload {
        id: String,
        is_running: bool,
      }
      let payload = RunningChangedPayload {
        id: updated_profile.id.to_string(),
        is_running: false,
      };

      if let Err(e) = events::emit("profile-running-changed", &payload) {
        log::warn!("Warning: Failed to emit profile running changed event: {e}");
      } else {
        log::info!(
          "Successfully emitted profile-running-changed event for Wayfern {}: running={}",
          updated_profile.name,
          payload.is_running
        );
      }

      log::info!(
        "Wayfern process cleanup completed for profile: {} (ID: {})",
        profile.name,
        profile.id
      );

      // Consolidate browser versions after stopping a browser
      if let Ok(consolidated) = self
        .downloaded_browsers_registry
        .consolidate_browser_versions(&app_handle)
      {
        if !consolidated.is_empty() {
          log::info!("Post-stop version consolidation results:");
          for action in &consolidated {
            log::info!("  {action}");
          }
        }
      }

      return Ok(());
    }

    // For non-camoufox/wayfern browsers, use the existing logic
    let profile_data_path = self.get_launch_profile_data_path(profile, profile.browser == "orbita");
    let profile_path_str = profile_data_path.to_string_lossy().to_string();

    let mut pids_to_kill = self.find_browser_processes_by_profile(profile);

    if let Some(stored_pid) = profile.process_id {
      let system = System::new_all();
      if let Some(process) = system.process(sysinfo::Pid::from(stored_pid as usize)) {
        let cmd: Vec<String> = process
          .cmd()
          .iter()
          .filter_map(|arg| arg.to_str().map(str::to_owned))
          .collect();
        let exe_name = process.name().to_string_lossy().to_string();

        if Self::process_matches_browser(&profile.browser, &exe_name, &cmd)
          && Self::process_matches_profile_path(&profile.browser, &cmd, &profile_path_str)
        {
          if !pids_to_kill.contains(&stored_pid) {
            pids_to_kill.push(stored_pid);
          }
        } else {
          log::info!(
            "Stored PID {} does not match active process info for profile {} (ID: {})",
            stored_pid,
            profile.name,
            profile.id
          );
        }
      } else {
        log::info!(
          "Stored PID {} is no longer valid for profile {} (ID: {})",
          stored_pid,
          profile.name,
          profile.id
        );
      }
    }

    pids_to_kill.sort_unstable();
    pids_to_kill.dedup();

    if pids_to_kill.is_empty() {
      return Err(
        format!(
          "No running {} browser process found for profile: {} (ID: {})",
          profile.browser, profile.name, profile.id
        )
        .into(),
      );
    }

    log::info!(
      "Attempting to kill browser processes {:?} for profile: {} (ID: {})",
      pids_to_kill,
      profile.name,
      profile.id
    );

    let profile_id_str = profile.id.to_string();
    if let Err(e) = PROXY_MANAGER
      .stop_proxy_by_profile_id(app_handle.clone(), &profile_id_str)
      .await
    {
      log::warn!(
        "Warning: Failed to stop proxy for profile {} before kill: {e}",
        profile_id_str
      );
    }

    for &pid in &pids_to_kill {
      if let Err(e) = PROXY_MANAGER.stop_proxy(app_handle.clone(), pid).await {
        log::warn!("Warning: Failed to stop proxy for PID {pid}: {e}");
      }

      #[cfg(target_os = "macos")]
      {
        platform_browser::macos::kill_browser_process_impl(pid, Some(&profile_path_str)).await?;
      }

      #[cfg(target_os = "windows")]
      {
        platform_browser::windows::kill_browser_process_impl(pid).await?;
      }

      #[cfg(target_os = "linux")]
      {
        platform_browser::linux::kill_browser_process_impl(pid).await?;
      }

      #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
      {
        return Err("Unsupported platform".into());
      }
    }

    let remaining_matching_pids = self.find_browser_processes_by_profile(profile);
    if !remaining_matching_pids.is_empty() {
      log::error!(
        "Browser processes {:?} are still running after kill attempt for profile: {} (ID: {})",
        remaining_matching_pids,
        profile.name,
        profile.id
      );
      return Err(
        format!(
          "Browser processes {:?} are still running after kill attempt",
          remaining_matching_pids
        )
        .into(),
      );
    }

    let system = System::new_all();
    let still_running_pids: Vec<u32> = pids_to_kill
      .iter()
      .copied()
      .filter(|pid| system.process(sysinfo::Pid::from(*pid as usize)).is_some())
      .collect();
    if !still_running_pids.is_empty() {
      log::error!(
        "Killed browser processes {:?} are still present for profile: {} (ID: {})",
        still_running_pids,
        profile.name,
        profile.id
      );
      return Err(
        format!(
          "Browser processes {:?} are still running after kill attempt",
          still_running_pids
        )
        .into(),
      );
    }

    log::info!(
      "Verified browser processes {:?} are terminated for profile: {} (ID: {})",
      pids_to_kill,
      profile.name,
      profile.id
    );

    // Clear the process ID from the profile
    let mut updated_profile = profile.clone();
    updated_profile.process_id = None;

    // Check for pending updates and apply them
    if let Ok(Some(pending_update)) = self
      .auto_updater
      .get_pending_update(&profile.browser, &profile.version)
    {
      log::info!(
        "Found pending update for profile {}: {} -> {}",
        profile.name,
        profile.version,
        pending_update.new_version
      );

      // Update the profile to the new version
      match self.profile_manager.update_profile_version(
        &app_handle,
        &profile.id.to_string(),
        &pending_update.new_version,
      ) {
        Ok(updated_profile_after_update) => {
          log::info!(
            "Successfully updated profile {} from version {} to {}",
            profile.name,
            profile.version,
            pending_update.new_version
          );
          updated_profile = updated_profile_after_update;

          // Remove the pending update from the auto updater state
          if let Err(e) = self
            .auto_updater
            .dismiss_update_notification(&pending_update.id)
          {
            log::warn!("Warning: Failed to dismiss pending update notification: {e}");
          }
        }
        Err(e) => {
          log::error!(
            "Failed to apply pending update for profile {}: {}",
            profile.name,
            e
          );
          // Continue with the original profile update (just clearing process_id)
        }
      }
    }

    self
      .save_process_info(&updated_profile)
      .map_err(|e| format!("Failed to update profile: {e}"))?;

    log::info!(
      "Emitting profile events for successful kill: {}",
      updated_profile.name
    );

    // Emit profile update event to frontend
    if let Err(e) = events::emit("profile-updated", &updated_profile) {
      log::warn!("Warning: Failed to emit profile update event: {e}");
    }

    // Emit minimal running changed event to frontend immediately
    #[derive(Serialize)]
    struct RunningChangedPayload {
      id: String,
      is_running: bool,
    }
    let payload = RunningChangedPayload {
      id: updated_profile.id.to_string(),
      is_running: false, // Explicitly set to false since we just killed it
    };

    if let Err(e) = events::emit("profile-running-changed", &payload) {
      log::warn!("Warning: Failed to emit profile running changed event: {e}");
    } else {
      log::info!(
        "Successfully emitted profile-running-changed event for {}: running={}",
        updated_profile.name,
        payload.is_running
      );
    }

    // Consolidate browser versions after stopping a browser
    if let Ok(consolidated) = self
      .downloaded_browsers_registry
      .consolidate_browser_versions(&app_handle)
    {
      if !consolidated.is_empty() {
        log::info!("Post-stop version consolidation results:");
        for action in &consolidated {
          log::info!("  {action}");
        }
      }
    }

    Ok(())
  }

  fn process_matches_browser(browser: &str, exe_name: &str, cmd: &[String]) -> bool {
    let exe_name_lower = exe_name.to_lowercase();

    match browser {
      "firefox" => {
        exe_name_lower.contains("firefox")
          && !exe_name_lower.contains("developer")
          && !exe_name_lower.contains("camoufox")
      }
      "firefox-developer" => {
        (exe_name_lower.contains("firefox") && exe_name_lower.contains("developer"))
          || (exe_name_lower.contains("firefox")
            && cmd.iter().any(|arg| {
              arg.contains("Developer")
                || arg.contains("developer")
                || arg.contains("FirefoxDeveloperEdition")
                || arg.contains("firefox-developer")
            }))
          || exe_name_lower == "firefox"
      }
      "zen" => exe_name_lower.contains("zen"),
      "chromium" => exe_name_lower.contains("chromium") || exe_name_lower.contains("chrome"),
      "brave" => exe_name_lower.contains("brave"),
      "orbita" => {
        exe_name_lower.contains("orbita")
          || exe_name_lower.contains("chromium")
          || exe_name_lower.contains("chrome")
      }
      "wayfern" => {
        exe_name_lower.contains("wayfern")
          || exe_name_lower.contains("chromium")
          || exe_name_lower.contains("chrome")
      }
      "cloakbrowser" => exe_name_lower.contains("chromium") || exe_name_lower.contains("chrome"),
      _ => false,
    }
  }

  fn process_matches_profile_path(browser: &str, cmd: &[String], profile_path: &str) -> bool {
    if matches!(browser, "firefox" | "firefox-developer" | "zen") {
      let mut found_profile_arg = false;
      for (index, arg) in cmd.iter().enumerate() {
        if arg == "-profile" && index + 1 < cmd.len() && cmd[index + 1] == profile_path {
          found_profile_arg = true;
          break;
        }

        if arg == &format!("-profile={profile_path}") || arg == profile_path {
          found_profile_arg = true;
          break;
        }
      }

      return found_profile_arg;
    }

    cmd.iter().any(|arg| {
      arg == &format!("--user-data-dir={profile_path}")
        || arg == profile_path
        || arg
          .strip_prefix("--user-data-dir=")
          .is_some_and(|value| value == profile_path)
    })
  }

  pub(crate) fn find_browser_processes_by_profile(&self, profile: &BrowserProfile) -> Vec<u32> {
    let system = System::new_all();
    let profiles_dir = self.profile_manager.get_profiles_dir();
    let profile_data_path = profile.get_profile_data_path(&profiles_dir);
    let profile_data_path_str = profile_data_path.to_string_lossy().to_string();

    let mut matching_pids = Vec::new();

    for (pid, process) in system.processes() {
      let cmd: Vec<String> = process
        .cmd()
        .iter()
        .filter_map(|arg| arg.to_str().map(str::to_owned))
        .collect();
      if cmd.is_empty() {
        continue;
      }

      let exe_name = process.name().to_string_lossy().to_string();
      if !Self::process_matches_browser(&profile.browser, &exe_name, &cmd) {
        continue;
      }

      if !Self::process_matches_profile_path(&profile.browser, &cmd, &profile_data_path_str) {
        continue;
      }

      matching_pids.push(pid.as_u32());
    }

    matching_pids.sort_unstable();
    matching_pids.dedup();

    matching_pids
  }

  pub async fn open_url_with_profile(
    &self,
    app_handle: tauri::AppHandle,
    profile_id: String,
    url: String,
  ) -> Result<(), String> {
    // Get the profile by name
    let profiles = self
      .profile_manager
      .list_profiles()
      .map_err(|e| format!("Failed to list profiles: {e}"))?;
    let profile = profiles
      .into_iter()
      .find(|p| p.id.to_string() == profile_id)
      .ok_or_else(|| format!("Profile '{profile_id}' not found"))?;

    log::info!("Opening URL '{url}' with profile '{profile_id}'");

    launch_browser_profile(app_handle, profile, Some(url.clone()))
      .await
      .map_err(|e| {
        log::info!("Failed to open URL with profile '{profile_id}': {e}");
        format!("Failed to open URL with profile: {e}")
      })?;

    log::info!("Successfully opened URL '{url}' with profile '{profile_id}'");
    Ok(())
  }
}

#[tauri::command]
pub async fn launch_browser_profile(
  app_handle: tauri::AppHandle,
  profile: BrowserProfile,
  url: Option<String>,
) -> Result<BrowserProfile, String> {
  log::info!(
    "Launch request received for profile: {} (ID: {})",
    profile.name,
    profile.id
  );

  let browser_runner = BrowserRunner::instance();

  // Store the internal proxy settings for passing to launch_browser
  let mut internal_proxy_settings: Option<ProxySettings> = None;

  // Resolve the most up-to-date profile from disk by ID to avoid using stale proxy_id/browser state
  log::info!(
    "launch_browser_profile called for profile: {} (ID: {}), has odoo_proxy: {}",
    profile.name,
    profile.id,
    profile.odoo_proxy.is_some()
  );

  let profile_for_launch = match browser_runner
    .profile_manager
    .list_profiles()
    .map_err(|e| format!("Failed to list profiles: {e}"))
  {
    Ok(profiles) => {
      let mut p = profiles
        .into_iter()
        .find(|p| p.id == profile.id)
        .unwrap_or_else(|| {
          log::info!("Profile not found on disk, using the one from frontend");
          profile.clone()
        });
      // Mergedynamic proxy from frontend if present
      if profile.odoo_proxy.is_some() {
        log::info!("Merging odoo_proxy from frontend into resolved profile");
        p.odoo_proxy = profile.odoo_proxy.clone();
      }
      p
    }
    Err(e) => {
      return Err(e);
    }
  };

  log::info!(
    "Resolved profile for launch: {} (ID: {})",
    profile_for_launch.name,
    profile_for_launch.id
  );

  // Always start a local proxy before launching (non-Camoufox/Wayfern handled here; they have their own flow)
  // This ensures all traffic goes through the local proxy for monitoring and future features
  if profile_for_launch.browser != "camoufox" && profile_for_launch.browser != "wayfern" {
    // Determine upstream proxy if configured; otherwise use DIRECT (no upstream)
    let upstream_proxy = browser_runner.resolve_upstream_proxy(&profile_for_launch);
    let has_upstream_proxy = upstream_proxy.is_some();

    // Use a temporary PID (1) to start the proxy, we'll update it after browser launch
    let temp_pid = 1u32;
    let profile_id_str = profile_for_launch.id.to_string();

    // Always start a local proxy, even if there's no upstream proxy
    // This allows for traffic monitoring and future features
    match PROXY_MANAGER
      .start_proxy(
        app_handle.clone(),
        upstream_proxy.as_ref(),
        temp_pid,
        Some(&profile_id_str),
      )
      .await
    {
      Ok(internal_proxy) => {
        if has_upstream_proxy {
          internal_proxy_settings = Some(internal_proxy.clone());
        }

        // For Firefox-based browsers, always apply PAC/user.js to point to the local proxy
        if matches!(
          profile_for_launch.browser.as_str(),
          "firefox" | "firefox-developer" | "zen"
        ) {
          let profiles_dir = browser_runner.profile_manager.get_profiles_dir();
          let profile_path = profiles_dir
            .join(profile_for_launch.id.to_string())
            .join("profile");

          // Provide a dummy upstream (ignored when internal proxy is provided)
          let dummy_upstream = ProxySettings {
            proxy_type: "http".to_string(),
            host: "127.0.0.1".to_string(),
            port: internal_proxy.port,
            username: None,
            password: None,
          };

          browser_runner
            .profile_manager
            .apply_proxy_settings_to_profile(&profile_path, &dummy_upstream, Some(&internal_proxy))
            .map_err(|e| format!("Failed to update profile proxy: {e}"))?;
        }

        log::info!(
          "Local proxy prepared for profile: {} on port: {} (upstream: {})",
          profile_for_launch.name,
          internal_proxy.port,
          upstream_proxy
            .as_ref()
            .map(|p| format!("{}:{}", p.host, p.port))
            .unwrap_or_else(|| "DIRECT".to_string())
        );
      }
      Err(e) => {
        let error_msg = format!("Failed to start local proxy: {e}");
        log::error!("{}", error_msg);
        // DO NOT launch browser if proxy startup fails - all browsers must use local proxy
        return Err(error_msg);
      }
    }
  }

  log::info!(
    "Starting browser launch for profile: {} (ID: {})",
    profile_for_launch.name,
    profile_for_launch.id
  );

  // Launch browser or open URL in existing instance
  let updated_profile = browser_runner.launch_or_open_url(app_handle.clone(), &profile_for_launch, url, internal_proxy_settings.as_ref()).await.map_err(|e| {
    log::info!("Browser launch failed for profile: {}, error: {}", profile_for_launch.name, e);

    // Emit a failure event to clear loading states in the frontend
    #[derive(serde::Serialize)]
    struct RunningChangedPayload {
      id: String,
      is_running: bool,
    }
    let payload = RunningChangedPayload {
      id: profile_for_launch.id.to_string(),
      is_running: false,
    };

    if let Err(emit_err) = events::emit("profile-running-changed", &payload) {
      log::warn!("Warning: Failed to emit profile running changed event: {emit_err}");
    }

    let error_str = e.to_string();

    // Check if the error is due to missing browser executable
    if error_str.contains("executable not found") || error_str.contains("does not exist on disk") {
      log::info!(
        "Missing browser detected for {}, checking registry for available versions",
        profile_for_launch.browser
      );

      // Check if another downloaded version exists in registry (version mismatch case)
      let downloaded_versions = browser_runner
        .downloaded_browsers_registry
        .get_downloaded_versions(&profile_for_launch.browser);

      if let Some(available_version) = downloaded_versions.into_iter().find(|v| {
        browser_runner
          .downloaded_browsers_registry
          .is_browser_downloaded(&profile_for_launch.browser, v)
      }) {
        if available_version != profile_for_launch.version {
          log::info!(
            "Found already-downloaded version {} for {} (profile had {}), updating profile version",
            available_version,
            profile_for_launch.browser,
            profile_for_launch.version
          );

          if let Err(e) = browser_runner.profile_manager.update_profile_version(
            &app_handle,
            &profile_for_launch.id.to_string(),
            &available_version,
          ) {
            log::warn!("Failed to update profile version: {e}");
          }

          return format!(
            "Browser version mismatch fixed. Profile updated to use downloaded version {}. Please try again.",
            available_version
          );
        }

        log::warn!(
          "Executable lookup failed for {} version {} although registry reports downloaded. Cleaning stale install and triggering re-download",
          profile_for_launch.browser,
          profile_for_launch.version
        );

        if let Err(clean_err) = browser_runner
          .downloaded_browsers_registry
          .cleanup_failed_download(&profile_for_launch.browser, &profile_for_launch.version)
        {
          log::warn!(
            "Failed to cleanup stale install for {} {}: {}",
            profile_for_launch.browser,
            profile_for_launch.version,
            clean_err
          );
        }
        if let Err(save_err) = browser_runner.downloaded_browsers_registry.save() {
          log::warn!("Failed to save registry after stale cleanup: {}", save_err);
        }

        let app_handle_clone = app_handle.clone();
        let browser_str = profile_for_launch.browser.clone();
        let version_str = profile_for_launch.version.clone();

        if crate::downloader::Downloader::instance().is_downloading(&browser_str, &version_str) {
          log::warn!(
            "Detected stale downloading state for {} {}. Clearing tracking and retrying.",
            profile_for_launch.browser,
            profile_for_launch.version
          );
          crate::downloader::Downloader::instance()
            .clear_download_tracking(&browser_str, &version_str);
        }

        tokio::spawn(async move {
          log::info!(
            "Auto re-download task started for {} {}",
            browser_str,
            version_str
          );
          let downloader = crate::downloader::Downloader::instance();
          let browser_for_event = browser_str.clone();
          let version_for_event = version_str.clone();

          if let Err(download_err) = downloader
            .download_browser_full(&app_handle_clone, browser_str, version_str)
            .await
          {
            log::error!("Auto re-download task failed before completion: {}", download_err);
            log::error!("Automatic re-download failed: {}", download_err);

            let failed_progress = crate::downloader::DownloadProgress {
              browser: browser_for_event,
              version: version_for_event,
              downloaded_bytes: 0,
              total_bytes: None,
              percentage: 0.0,
              speed_bytes_per_sec: 0.0,
              eta_seconds: None,
              stage: "cancelled".to_string(),
            };
            let _ = crate::events::emit("download-progress", &failed_progress);
          }
        });

        return format!(
          "Trình duyệt {} phiên bản {} bị lỗi hoặc thiếu file thực thi. Đang tự động tải lại, vui lòng đợi tải xong rồi thử lại.",
          profile_for_launch.browser,
          profile_for_launch.version
        );
      }

      let app_handle_clone = app_handle.clone();
      let browser_str = profile_for_launch.browser.clone();
      let version_str = profile_for_launch.version.clone();

      if crate::downloader::Downloader::instance().is_downloading(&browser_str, &version_str) {
        log::warn!(
          "Detected stale downloading state for {} {}. Clearing tracking and retrying.",
          profile_for_launch.browser,
          profile_for_launch.version
        );
        crate::downloader::Downloader::instance()
          .clear_download_tracking(&browser_str, &version_str);
      }

      // Trigger download in background
      tokio::spawn(async move {
        let downloader = crate::downloader::Downloader::instance();
        let browser_for_event = browser_str.clone();
        let version_for_event = version_str.clone();

        if let Err(download_err) = downloader
          .download_browser_full(&app_handle_clone, browser_str, version_str)
          .await
        {
          log::error!("Automated background download failed: {}", download_err);

          let failed_progress = crate::downloader::DownloadProgress {
            browser: browser_for_event,
            version: version_for_event,
            downloaded_bytes: 0,
            total_bytes: None,
            percentage: 0.0,
            speed_bytes_per_sec: 0.0,
            eta_seconds: None,
            stage: "cancelled".to_string(),
          };
          let _ = crate::events::emit("download-progress", &failed_progress);
        }
      });

      return format!(
        "Browser {} is not downloaded. Starting download now. Please wait and try again once the download completes.",
        profile_for_launch.browser
      );
    }

    // Check if this is an architecture compatibility issue
    if let Some(io_error) = e.downcast_ref::<std::io::Error>() {
      if io_error.kind() == std::io::ErrorKind::Other && io_error.to_string().contains("Exec format error") {
        return format!("Failed to launch browser: Executable format error. This browser version is not compatible with your system architecture ({}). Please try a different browser or version that supports your platform.", std::env::consts::ARCH);
      }
    }
    format!("Failed to launch browser or open URL: {e}")
  })?;

  log::info!(
    "Browser launch completed for profile: {} (ID: {})",
    updated_profile.name,
    updated_profile.id
  );

  // Now update the proxy with the correct PID if we have one
  if let Some(actual_pid) = updated_profile.process_id {
    // Update the proxy manager with the correct PID (we always started with temp pid 1 for non-Camoufox)
    let _ = PROXY_MANAGER.update_proxy_pid(1u32, actual_pid);
  }

  Ok(updated_profile)
}

#[tauri::command]
pub fn check_browser_exists(browser_str: String, version: String) -> bool {
  // This is an alias for is_browser_downloaded to provide clearer semantics for auto-updates
  let runner = BrowserRunner::instance();
  runner
    .downloaded_browsers_registry
    .is_browser_downloaded(&browser_str, &version)
}

#[tauri::command]
pub async fn kill_browser_profile(
  app_handle: tauri::AppHandle,
  profile: BrowserProfile,
) -> Result<(), String> {
  log::info!(
    "Kill request received for profile: {} (ID: {})",
    profile.name,
    profile.id
  );

  let browser_runner = BrowserRunner::instance();

  match browser_runner
    .kill_browser_process(app_handle.clone(), &profile)
    .await
  {
    Ok(()) => {
      log::info!(
        "Successfully killed browser profile: {} (ID: {})",
        profile.name,
        profile.id
      );
      Ok(())
    }
    Err(e) => {
      log::info!("Failed to kill browser profile {}: {}", profile.name, e);

      // Emit a failure event to clear loading states in the frontend
      #[derive(serde::Serialize)]
      struct RunningChangedPayload {
        id: String,
        is_running: bool,
      }
      // If the error is just that the process is not found, we should consider it
      // successfully killed (since it's already dead) and update UI state to false.
      let is_already_dead =
        e.to_string().contains("No running") && e.to_string().contains("browser process found");

      let payload = RunningChangedPayload {
        id: profile.id.to_string(),
        is_running: !is_already_dead,
      };

      if let Err(emit_err) = events::emit("profile-running-changed", &payload) {
        log::warn!("Warning: Failed to emit profile running changed event: {emit_err}");
      }

      if is_already_dead {
        log::info!("Browser process for {} was already closed.", profile.name);

        // Clear the process ID from the profile to ensure consistent state
        let mut updated_profile = profile.clone();
        updated_profile.process_id = None;
        let _ = browser_runner.save_process_info(&updated_profile);

        if let Err(emit_err) = events::emit("profile-updated", &updated_profile) {
          log::warn!("Warning: Failed to emit profile update event: {emit_err}");
        }

        return Ok(());
      }

      Err(format!("Failed to kill browser: {e}"))
    }
  }
}

pub async fn launch_browser_profile_with_debugging(
  app_handle: tauri::AppHandle,
  profile: BrowserProfile,
  url: Option<String>,
  remote_debugging_port: Option<u16>,
  headless: bool,
) -> Result<BrowserProfile, String> {
  let browser_runner = BrowserRunner::instance();
  browser_runner
    .launch_browser_with_debugging(app_handle, &profile, url, remote_debugging_port, headless)
    .await
    .map_err(|e| format!("Failed to launch browser with debugging: {e}"))
}

#[tauri::command]
pub async fn open_url_with_profile(
  app_handle: tauri::AppHandle,
  profile_id: String,
  url: String,
) -> Result<(), String> {
  let browser_runner = BrowserRunner::instance();
  browser_runner
    .open_url_with_profile(app_handle, profile_id, url)
    .await
}

// Global singleton instance
lazy_static::lazy_static! {
  static ref BROWSER_RUNNER: BrowserRunner = BrowserRunner::new();
}
