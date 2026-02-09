use crate::camoufox_manager::CamoufoxConfig;
use crate::wayfern_manager::WayfernConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum SyncStatus {
  #[default]
  Disabled,
  Syncing,
  Synced,
  Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrowserProfile {
  pub id: uuid::Uuid,
  pub name: String,
  pub browser: String,
  pub version: String,
  #[serde(default)]
  pub proxy_id: Option<String>, // Reference to stored proxy
  #[serde(default)]
  pub process_id: Option<u32>,
  #[serde(default)]
  pub last_launch: Option<u64>,
  #[serde(default = "default_release_type")]
  pub release_type: String, // "stable" or "nightly"
  #[serde(default)]
  pub camoufox_config: Option<CamoufoxConfig>, // Camoufox configuration
  #[serde(default)]
  pub wayfern_config: Option<WayfernConfig>, // Wayfern configuration
  #[serde(default)]
  pub group_id: Option<String>, // Reference to profile group
  #[serde(default)]
  pub tags: Vec<String>, // Free-form tags
  #[serde(default)]
  pub note: Option<String>, // User note
  #[serde(default)]
  pub sync_enabled: bool,
  #[serde(default)]
  pub last_sync: Option<u64>, // Timestamp of last successful sync (epoch seconds)
  #[serde(default)]
  pub odoo_id: Option<String>,
  #[serde(default)]
  pub profile_url: Option<String>,
  #[serde(default)]
  pub created_at: Option<u64>, // Creation timestamp in epoch seconds
  #[serde(default)]
  pub odoo_proxy: Option<crate::odoo::types::OdooProxy>,
  #[serde(default)]
  pub username: Option<String>,
  #[serde(default)]
  pub password: Option<String>,
  #[serde(default)]
  pub user_agent: Option<String>,
  #[serde(skip_deserializing)]
  pub absolute_path: Option<String>,
}

pub fn default_release_type() -> String {
  "stable".to_string()
}

impl BrowserProfile {
  /// Get the path to the profile data directory (profiles/{uuid}/profile)
  pub fn get_profile_data_path(&self, profiles_dir: &Path) -> PathBuf {
    profiles_dir.join(self.id.to_string()).join("profile")
  }
}
