use crate::browser::{BrowserType, ProxySettings};
use crate::camoufox_manager::CamoufoxConfig;
use crate::events;
use crate::profile::types::BrowserProfile;
use crate::wayfern_manager::WayfernConfig;
use directories::BaseDirs;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};

pub struct ProfileManager {
  base_dirs: BaseDirs,
  camoufox_manager: &'static crate::camoufox_manager::CamoufoxManager,
  wayfern_manager: &'static crate::wayfern_manager::WayfernManager,
}

impl ProfileManager {
  fn new() -> Self {
    Self {
      base_dirs: BaseDirs::new().expect("Failed to get base directories"),
      camoufox_manager: crate::camoufox_manager::CamoufoxManager::instance(),
      wayfern_manager: crate::wayfern_manager::WayfernManager::instance(),
    }
  }

  pub fn instance() -> &'static ProfileManager {
    &PROFILE_MANAGER
  }

  pub fn get_profiles_dir(&self) -> PathBuf {
    let mut path = self.base_dirs.data_local_dir().to_path_buf();
    path.push(if cfg!(debug_assertions) {
      "FoxiaDev"
    } else {
      "Foxia"
    });
    path.push("profiles");
    path
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

  #[allow(clippy::too_many_arguments)]
  pub async fn create_profile_with_group(
    &self,
    app_handle: &tauri::AppHandle,
    name: &str,
    browser: &str,
    version: &str,
    release_type: &str,
    proxy_id: Option<String>,
    camoufox_config: Option<CamoufoxConfig>,
    wayfern_config: Option<WayfernConfig>,
    group_id: Option<String>,
    username: Option<String>,
    password: Option<String>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    log::info!("Attempting to create profile: {name}");

    let existing_profiles = self.list_profiles()?;
    if existing_profiles
      .iter()
      .any(|p| p.name.to_lowercase() == name.to_lowercase())
    {
      return Err(format!("Profile with name '{name}' already exists").into());
    }

    let profile_id = uuid::Uuid::new_v4();
    let profiles_dir = self.get_profiles_dir();
    let profile_uuid_dir = profiles_dir.join(profile_id.to_string());
    let profile_data_dir = profile_uuid_dir.join("profile");
    create_dir_all(&profile_uuid_dir)?;
    create_dir_all(&profile_data_dir)?;

    let mut user_agent = None;

    let final_camoufox_config = if browser == "camoufox" {
      let mut config = camoufox_config.unwrap_or_default();
      if config.executable_path.is_none() {
        let mut browser_dir = self.get_binaries_dir();
        browser_dir.push(browser);
        browser_dir.push(version);
        #[cfg(target_os = "macos")]
        let binary_path = browser_dir.join("Camoufox.app/Contents/MacOS/camoufox");
        #[cfg(target_os = "windows")]
        let binary_path = browser_dir.join("camoufox.exe");
        #[cfg(target_os = "linux")]
        let binary_path = browser_dir.join("camoufox");
        config.executable_path = Some(binary_path.to_string_lossy().to_string());
      }

      if config.fingerprint.is_none() {
        let temp_profile = BrowserProfile {
          id: uuid::Uuid::new_v4(),
          name: name.to_string(),
          browser: browser.to_string(),
          version: version.to_string(),
          proxy_id: proxy_id.clone(),
          process_id: None,
          last_launch: None,
          release_type: release_type.to_string(),
          camoufox_config: None,
          wayfern_config: None,
          group_id: group_id.clone(),
          tags: Vec::new(),
          note: None,
          sync_enabled: false,
          last_sync: None,
          odoo_id: None,
          profile_url: None,
          created_at: Some(chrono::Utc::now().timestamp() as u64),
          odoo_proxy: None,
          username: username.clone(),
          password: password.clone(),
          user_agent: None,
          absolute_path: None,
        };
        if let Ok(gen_fp) = self
          .camoufox_manager
          .generate_fingerprint_config(app_handle, &temp_profile, &config)
          .await
        {
          // Extract User Agent from Camoufox fingerprint
          if let Ok(fp_val) = serde_json::from_str::<serde_json::Value>(&gen_fp) {
            if let Some(ua) = fp_val.get("headers.User-Agent").and_then(|v| v.as_str()) {
              user_agent = Some(ua.to_string());
            } else if let Some(ua) = fp_val.get("navigator.userAgent").and_then(|v| v.as_str()) {
              user_agent = Some(ua.to_string());
            }
          }
          config.fingerprint = Some(gen_fp);
        }
      } else if let Some(fp_str) = &config.fingerprint {
        // Extract User Agent from existing fingerprint
        if let Ok(fp_val) = serde_json::from_str::<serde_json::Value>(fp_str) {
          if let Some(ua) = fp_val.get("headers.User-Agent").and_then(|v| v.as_str()) {
            user_agent = Some(ua.to_string());
          }
        }
      }
      config.proxy = None;
      Some(config)
    } else {
      camoufox_config
    };

    let final_wayfern_config = if browser == "wayfern" {
      let mut config = wayfern_config.unwrap_or_default();
      if config.executable_path.is_none() {
        let mut browser_dir = self.get_binaries_dir();
        browser_dir.push(browser);
        browser_dir.push(version);
        #[cfg(target_os = "macos")]
        let binary_path = browser_dir.join("Chromium.app/Contents/MacOS/Chromium");
        #[cfg(target_os = "windows")]
        let binary_path = browser_dir.join("chrome.exe");
        #[cfg(target_os = "linux")]
        let binary_path = browser_dir.join("chrome");
        config.executable_path = Some(binary_path.to_string_lossy().to_string());
      }

      if config.fingerprint.is_none() {
        let temp_profile = BrowserProfile {
          id: uuid::Uuid::new_v4(),
          name: name.to_string(),
          browser: browser.to_string(),
          version: version.to_string(),
          proxy_id: proxy_id.clone(),
          process_id: None,
          last_launch: None,
          release_type: release_type.to_string(),
          camoufox_config: None,
          wayfern_config: None,
          group_id: group_id.clone(),
          tags: Vec::new(),
          note: None,
          sync_enabled: false,
          last_sync: None,
          odoo_id: None,
          profile_url: None,
          created_at: Some(chrono::Utc::now().timestamp() as u64),
          odoo_proxy: None,
          username: username.clone(),
          password: password.clone(),
          user_agent: None,
          absolute_path: None,
        };
        if let Ok(gen_fp) = self
          .wayfern_manager
          .generate_fingerprint_config(app_handle, &temp_profile, &config)
          .await
        {
          // Extract User Agent from Wayfern fingerprint
          if let Ok(fp_val) = serde_json::from_str::<serde_json::Value>(&gen_fp) {
            if let Some(ua) = fp_val.get("userAgent").and_then(|v| v.as_str()) {
              user_agent = Some(ua.to_string());
            }
          }
          config.fingerprint = Some(gen_fp);
        }
      } else if let Some(fp_str) = &config.fingerprint {
        // Extract User Agent from existing fingerprint
        if let Ok(fp_val) = serde_json::from_str::<serde_json::Value>(fp_str) {
          if let Some(ua) = fp_val.get("userAgent").and_then(|v| v.as_str()) {
            user_agent = Some(ua.to_string());
          }
        }
      }
      config.proxy = None;
      Some(config)
    } else {
      wayfern_config
    };

    let profile = BrowserProfile {
      id: profile_id,
      name: name.to_string(),
      browser: browser.to_string(),
      version: version.to_string(),
      proxy_id: proxy_id.clone(),
      process_id: None,
      last_launch: None,
      release_type: release_type.to_string(),
      camoufox_config: final_camoufox_config,
      wayfern_config: final_wayfern_config,
      group_id: group_id.clone(),
      tags: Vec::new(),
      note: None,
      sync_enabled: false,
      last_sync: None,
      odoo_id: None,
      profile_url: None,
      created_at: Some(chrono::Utc::now().timestamp() as u64),
      odoo_proxy: None,
      username,
      password,
      user_agent,
      absolute_path: None,
    };

    self.save_profile(&profile)?;
    self.disable_proxy_settings_in_profile(&profile_data_dir)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn save_profile(&self, profile: &BrowserProfile) -> Result<(), Box<dyn std::error::Error>> {
    let profiles_dir = self.get_profiles_dir();
    let profile_uuid_dir = profiles_dir.join(profile.id.to_string());
    create_dir_all(&profile_uuid_dir)?;
    let json = serde_json::to_string_pretty(profile)?;
    fs::write(profile_uuid_dir.join("metadata.json"), json)?;
    Ok(())
  }

  pub fn list_profiles(&self) -> Result<Vec<BrowserProfile>, Box<dyn std::error::Error>> {
    let profiles_dir = self.get_profiles_dir();
    if !profiles_dir.exists() {
      return Ok(vec![]);
    }
    let mut profiles = Vec::new();
    for entry in fs::read_dir(profiles_dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_dir() {
        let meta = path.join("metadata.json");
        if meta.exists() {
          let content = fs::read_to_string(&meta)?;
          if let Ok(mut p) = serde_json::from_str::<BrowserProfile>(&content) {
            p.absolute_path = Some(path.join("profile").to_string_lossy().to_string());
            profiles.push(p);
          }
        }
      }
    }
    Ok(profiles)
  }

  pub fn rename_profile(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    new_name: &str,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.name = new_name.to_string();
    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn update_profile_details(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    name: String,
    username: Option<String>,
    password: Option<String>,
    user_agent: Option<String>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;

    profile.name = name;
    profile.username = username;
    profile.password = password;
    profile.user_agent = user_agent;

    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn delete_profile(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let profiles_dir = self.get_profiles_dir();
    let profile_uuid_dir = profiles_dir.join(profile_uuid.to_string());
    if profile_uuid_dir.exists() {
      fs::remove_dir_all(&profile_uuid_dir)?;
    }
    let _ = events::emit_empty("profiles-changed");
    Ok(())
  }

  pub async fn update_camoufox_config(
    &self,
    _app_handle: tauri::AppHandle,
    profile_id: &str,
    config: CamoufoxConfig,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id).map_err(|e| e.to_string())?;
    let mut profile = self
      .list_profiles()
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.camoufox_config = Some(config);
    self.save_profile(&profile).map_err(|e| e.to_string())?;
    let _ = events::emit_empty("profiles-changed");
    Ok(())
  }

  pub async fn update_wayfern_config(
    &self,
    _app_handle: tauri::AppHandle,
    profile_id: &str,
    config: WayfernConfig,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id).map_err(|e| e.to_string())?;
    let mut profile = self
      .list_profiles()
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.wayfern_config = Some(config);
    self.save_profile(&profile).map_err(|e| e.to_string())?;
    let _ = events::emit_empty("profiles-changed");
    Ok(())
  }

  pub async fn update_profile_proxy(
    &self,
    _app_handle: tauri::AppHandle,
    profile_id: &str,
    proxy_id: Option<String>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error + Send + Sync>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id).map_err(|e| e.to_string())?;
    let mut profile = self
      .list_profiles()
      .map_err(|e| e.to_string())?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.proxy_id = proxy_id;
    self.save_profile(&profile).map_err(|e| e.to_string())?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn update_profile_tags(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    tags: Vec<String>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.tags = tags;
    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn update_profile_note(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    note: Option<String>,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.note = note;
    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn update_profile_url(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    profile_url: String,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.profile_url = Some(profile_url);
    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub fn update_profile_odoo_id(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    odoo_id: String,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.odoo_id = Some(odoo_id);
    self.save_profile(&profile)?;
    let _ = events::emit_empty("profiles-changed");
    Ok(profile)
  }

  pub async fn check_browser_status(
    &self,
    _app_handle: tauri::AppHandle,
    profile: &BrowserProfile,
  ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if profile.browser == "camoufox" {
      let launcher = self.camoufox_manager;
      let profiles_dir = self.get_profiles_dir();
      let profile_data_path = profile.get_profile_data_path(&profiles_dir);
      let path_str = profile_data_path.to_string_lossy();
      match launcher.find_camoufox_by_profile(&path_str).await {
        Ok(Some(_)) => Ok(true),
        _ => Ok(false),
      }
    } else {
      Ok(profile.process_id.is_some())
    }
  }

  pub fn clone_profile(
    &self,
    profile_id: &str,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let source = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    let new_id = uuid::Uuid::new_v4();
    let mut new_profile = source.clone();
    new_profile.id = new_id;
    new_profile.name = format!("{} (Copy)", source.name);
    new_profile.created_at = Some(chrono::Utc::now().timestamp() as u64);
    self.save_profile(&new_profile)?;
    Ok(new_profile)
  }

  pub async fn import_zsmkt_profiles_batch(
    &self,
    _app_handle: &tauri::AppHandle,
    zs_profiles: Vec<crate::profile::zsmkt_import::ZsMktProfile>,
  ) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    let existing_profiles = self.list_profiles()?;

    for zs in zs_profiles {
      let profile = crate::profile::zsmkt_import::convert_zsmkt_profile(zs, None);

      // Check if profile already exists (by UUID or odoo_id)
      let already_exists = existing_profiles.iter().any(|p| {
        p.id == profile.id
          || (p.odoo_id.is_some() && profile.odoo_id.is_some() && p.odoo_id == profile.odoo_id)
      });

      if already_exists {
        log::info!(
          "Profile '{}' (UUID: {}) already exists, skipping import",
          profile.name,
          profile.id
        );
        continue;
      }

      if self.save_profile(&profile).is_ok() {
        log::info!(
          "Imported new profile '{}' (UUID: {})",
          profile.name,
          profile.id
        );
        count += 1;
      }
    }
    if count > 0 {
      let _ = events::emit_empty("profiles-changed");
    }
    Ok(count)
  }

  pub fn apply_proxy_settings_to_profile(
    &self,
    path: &Path,
    proxy: &ProxySettings,
    internal: Option<&ProxySettings>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let user_js = path.join("user.js");
    let p = internal.unwrap_or(proxy);

    let prefs = vec![
      format!("user_pref(\"network.proxy.type\", 1);"),
      format!("user_pref(\"network.proxy.share_proxy_settings\", true);"),
      format!("user_pref(\"network.proxy.http\", \"{}\");", p.host),
      format!("user_pref(\"network.proxy.http_port\", {});", p.port),
      format!("user_pref(\"network.proxy.ssl\", \"{}\");", p.host),
      format!("user_pref(\"network.proxy.ssl_port\", {});", p.port),
      format!("user_pref(\"network.proxy.socks\", \"{}\");", p.host),
      format!("user_pref(\"network.proxy.socks_port\", {});", p.port),
      format!("user_pref(\"network.proxy.socks_remote_dns\", true);"),
      format!("user_pref(\"network.proxy.no_proxies_on\", \"localhost, 127.0.0.1\");"),
    ];

    fs::write(user_js, prefs.join("\n"))?;
    Ok(())
  }

  pub fn disable_proxy_settings_in_profile(
    &self,
    path: &Path,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let user_js = path.join("user.js");
    fs::write(user_js, "user_pref(\"network.proxy.type\", 0);")?;
    Ok(())
  }

  pub fn update_profile_version(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_id: &str,
    version: &str,
  ) -> Result<BrowserProfile, Box<dyn std::error::Error>> {
    let profile_uuid = uuid::Uuid::parse_str(profile_id)?;
    let mut profile = self
      .list_profiles()?
      .into_iter()
      .find(|p| p.id == profile_uuid)
      .ok_or("Profile not found")?;
    profile.version = version.to_string();
    self.save_profile(&profile)?;
    Ok(profile)
  }

  pub fn assign_profiles_to_group(
    &self,
    _app_handle: &tauri::AppHandle,
    profile_ids: Vec<String>,
    group_id: Option<String>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = self.list_profiles()?;
    for id in profile_ids {
      let uuid = uuid::Uuid::parse_str(&id)?;
      if let Some(mut p) = profiles.iter().find(|p| p.id == uuid).cloned() {
        p.group_id = group_id.clone();
        self.save_profile(&p)?;
      }
    }
    let _ = events::emit_empty("profiles-changed");
    Ok(())
  }

  pub fn delete_multiple_profiles(
    &self,
    app_handle: &tauri::AppHandle,
    profile_ids: Vec<String>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    for id in profile_ids {
      let _ = self.delete_profile(app_handle, &id);
    }
    Ok(())
  }
}

lazy_static::lazy_static! {
  static ref PROFILE_MANAGER: ProfileManager = ProfileManager::new();
}

#[tauri::command]
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn create_browser_profile_with_group(
  app_handle: tauri::AppHandle,
  name: String,
  browser: String,
  version: String,
  release_type: String,
  proxy_id: Option<String>,
  camoufox_config: Option<CamoufoxConfig>,
  wayfern_config: Option<WayfernConfig>,
  group_id: Option<String>,
  username: Option<String>,
  password: Option<String>,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .create_profile_with_group(
      &app_handle,
      &name,
      &browser,
      &version,
      &release_type,
      proxy_id,
      camoufox_config,
      wayfern_config,
      group_id,
      username,
      password,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_browser_profiles() -> Result<Vec<BrowserProfile>, String> {
  ProfileManager::instance()
    .list_profiles()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_profile_proxy(
  app_handle: tauri::AppHandle,
  profile_id: String,
  proxy_id: Option<String>,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_proxy(app_handle, &profile_id, proxy_id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_tags(
  app_handle: tauri::AppHandle,
  profile_id: String,
  tags: Vec<String>,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_tags(&app_handle, &profile_id, tags)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_note(
  app_handle: tauri::AppHandle,
  profile_id: String,
  note: Option<String>,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_note(&app_handle, &profile_id, note)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_url(
  app_handle: tauri::AppHandle,
  profile_id: String,
  profile_url: String,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_url(&app_handle, &profile_id, profile_url)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_odoo_id(
  app_handle: tauri::AppHandle,
  profile_id: String,
  odoo_id: String,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_odoo_id(&app_handle, &profile_id, odoo_id)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_browser_status(
  app_handle: tauri::AppHandle,
  profile: BrowserProfile,
) -> Result<bool, String> {
  ProfileManager::instance()
    .check_browser_status(app_handle, &profile)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_profile(
  app_handle: tauri::AppHandle,
  profile_id: String,
  new_name: String,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .rename_profile(&app_handle, &profile_id, &new_name)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_details(
  app_handle: tauri::AppHandle,
  profile_id: String,
  name: String,
  username: Option<String>,
  password: Option<String>,
  user_agent: Option<String>,
) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .update_profile_details(&app_handle, &profile_id, name, username, password, user_agent)
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn create_browser_profile_new(
  app_handle: tauri::AppHandle,
  name: String,
  browser_str: String,
  version: String,
  release_type: String,
  proxy_id: Option<String>,
  camoufox_config: Option<CamoufoxConfig>,
  wayfern_config: Option<WayfernConfig>,
  group_id: Option<String>,
  username: Option<String>,
  password: Option<String>,
) -> Result<BrowserProfile, String> {
  let browser_type = BrowserType::from_str(&browser_str).map_err(|e| e.to_string())?;
  ProfileManager::instance()
    .create_profile_with_group(
      &app_handle,
      &name,
      browser_type.as_str(),
      &version,
      &release_type,
      proxy_id,
      camoufox_config,
      wayfern_config,
      group_id,
      username,
      password,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_camoufox_config(
  app_handle: tauri::AppHandle,
  profile_id: String,
  config: CamoufoxConfig,
) -> Result<(), String> {
  ProfileManager::instance()
    .update_camoufox_config(app_handle, &profile_id, config)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_wayfern_config(
  app_handle: tauri::AppHandle,
  profile_id: String,
  config: WayfernConfig,
) -> Result<(), String> {
  ProfileManager::instance()
    .update_wayfern_config(app_handle, &profile_id, config)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clone_profile(profile_id: String) -> Result<BrowserProfile, String> {
  ProfileManager::instance()
    .clone_profile(&profile_id)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(app_handle: tauri::AppHandle, profile_id: String) -> Result<(), String> {
  ProfileManager::instance()
    .delete_profile(&app_handle, &profile_id)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_zsmkt_profiles_batch(
  app_handle: tauri::AppHandle,
  zs_profiles: Vec<crate::profile::zsmkt_import::ZsMktProfile>,
) -> Result<usize, String> {
  ProfileManager::instance()
    .import_zsmkt_profiles_batch(&app_handle, zs_profiles)
    .await
    .map_err(|e| e.to_string())
}
