use crate::camoufox_manager::CamoufoxConfig;
use crate::profile::types::{default_release_type, BrowserProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZsMktFingerprint {
  #[serde(rename = "userAgent")]
  pub user_agent: String,
  pub timezone: String,
  pub language: String,
  pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZsMktProxy {
  pub protocol: String,
  pub host: String,
  pub port: serde_json::Value, // Can be number or string in JSON
  pub username: Option<String>,
  pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZsMktProfile {
  pub id: String,
  pub name: String,
  pub fingerprint: ZsMktFingerprint,
  pub status: String,
  #[serde(rename = "localPath")]
  pub local_path: String,
  pub proxy: Option<ZsMktProxy>,
  #[serde(rename = "createdAt")]
  pub created_at_str: Option<String>,
  #[serde(rename = "profileUrl")]
  pub profile_url: Option<String>,
  pub username: Option<String>,
  pub password: Option<String>,
  pub browser: Option<String>,
}

/// Convert a zs-mkt proxy to a Foxia proxy settings.
#[allow(dead_code)]
pub fn convert_zsmkt_proxy(zs_proxy: &ZsMktProxy) -> crate::browser::ProxySettings {
  let port = match &zs_proxy.port {
    serde_json::Value::Number(n) => n.as_u64().unwrap_or(8080) as u16,
    serde_json::Value::String(s) => s.parse::<u16>().unwrap_or(8080),
    _ => 8080,
  };

  crate::browser::ProxySettings {
    proxy_type: zs_proxy.protocol.clone(),
    host: zs_proxy.host.clone(),
    port,
    username: zs_proxy.username.clone(),
    password: zs_proxy.password.clone(),
  }
}

/// Convert a zs-mkt profile to a Foxia profile.
pub fn convert_zsmkt_profile(zs_profile: ZsMktProfile, proxy_id: Option<String>) -> BrowserProfile {
  // Try to parse as UUID first, otherwise create deterministic UUID from odoo_id
  let id = Uuid::parse_str(&zs_profile.id).unwrap_or_else(|_| {
    // Create UUID v5 from odoo_id for deterministic ID generation
    let namespace = Uuid::NAMESPACE_OID;
    let name = format!("odoo-profile-{}", zs_profile.id);
    Uuid::new_v5(&namespace, name.as_bytes())
  });

  log::info!(
    "Converting zs-mkt profile '{}' (odoo_id: {}) -> UUID: {} with profile_url: {:?}",
    zs_profile.name,
    zs_profile.id,
    id,
    zs_profile.profile_url
  );

  // Construct fingerprint JSON for Camoufox
  let mut fp_config = HashMap::new();
  fp_config.insert(
    "navigator.userAgent".to_string(),
    serde_json::json!(zs_profile.fingerprint.user_agent),
  );
  fp_config.insert(
    "timezone".to_string(),
    serde_json::json!(zs_profile.fingerprint.timezone),
  );
  fp_config.insert(
    "navigator.language".to_string(),
    serde_json::json!(zs_profile.fingerprint.language),
  );

  if let Some(platform) = &zs_profile.fingerprint.platform {
    fp_config.insert(
      "navigator.platform".to_string(),
      serde_json::json!(platform),
    );
  }

  let fingerprint_json = serde_json::to_string(&fp_config).unwrap_or_default();

  // Construct CamoufoxConfig
  let camoufox_config = CamoufoxConfig {
    fingerprint: Some(fingerprint_json),
    os: zs_profile.fingerprint.platform.clone(),
    ..Default::default()
  };

  // Determine the version.
  let version = "v135.0.1-beta.24".to_string();

  let created_at = zs_profile.created_at_str.and_then(|s| {
    log::info!("Parsing createdAt: '{}'", s);

    // Try multiple formats
    // 1. RFC3339 with timezone: "2026-02-02T09:46:36.450110Z"
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", s)) {
      let timestamp = dt.timestamp() as u64;
      log::info!("Parsed as RFC3339+Z: {} -> {}", s, timestamp);
      return Some(timestamp);
    }

    // 2. RFC3339 without timezone, assume UTC: "2026-02-02T09:46:36.450110"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
      let timestamp = dt.and_utc().timestamp() as u64;
      log::info!("Parsed as ISO8601: {} -> {}", s, timestamp);
      return Some(timestamp);
    }

    // 3. Space-separated format: "2026-02-02 09:46:36.450110"
    let normalized = s.replace(" ", "T");
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.f") {
      let timestamp = dt.and_utc().timestamp() as u64;
      log::info!("Parsed as space-separated: {} -> {}", s, timestamp);
      return Some(timestamp);
    }

    // 4. Simple datetime without microseconds: "2026-02-02 09:46:36"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
      let timestamp = dt.and_utc().timestamp() as u64;
      log::info!("Parsed as simple datetime: {} -> {}", s, timestamp);
      return Some(timestamp);
    }

    log::warn!("Failed to parse createdAt: {}", s);
    None
  });

  BrowserProfile {
    id,
    name: zs_profile.name,
    browser: zs_profile.browser.unwrap_or_else(|| "camoufox".to_string()),
    version,
    proxy_id,
    process_id: None,
    last_launch: None,
    release_type: default_release_type(),
    camoufox_config: Some(camoufox_config),
    wayfern_config: None,
    group_id: None,
    tags: Vec::new(),
    note: Some(format!(
      "Imported from zs-mkt (original path: {})",
      zs_profile.local_path
    )),
    sync_enabled: false,
    last_sync: None,
    odoo_id: Some(zs_profile.id),
    profile_url: zs_profile.profile_url,
    created_at: created_at.or(Some(chrono::Utc::now().timestamp() as u64)),
    odoo_proxy: None,
    username: zs_profile.username,
    password: zs_profile.password,
    user_agent: None,
    absolute_path: None,
  }
}
