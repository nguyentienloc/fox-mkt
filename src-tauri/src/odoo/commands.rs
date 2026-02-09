use crate::events;
use crate::odoo::client::OdooClient;
use crate::odoo::types::*;
use crate::odoo::ODOO_CLIENT;
use crate::profile::manager::ProfileManager;
use crate::s3_transfer;
use tauri::Emitter;

#[tauri::command]
pub async fn odoo_login(
  base_url: String,
  login: String,
  password: String,
) -> Result<OdooLoginResult, String> {
  let client = OdooClient::new(base_url);
  let result = client
    .login(login, password)
    .await
    .map_err(|e| e.to_string())?;

  log::info!("Login successful! session_id: {:?}", result.session_id);

  let mut odoo_client = ODOO_CLIENT.lock().await;
  *odoo_client = Some(client);

  Ok(result)
}

#[tauri::command]
pub async fn upload_profile_to_odoo_s3(
  _app_handle: tauri::AppHandle,
  profile_id: String,
  base_url: String,
  session_id: String,
) -> Result<String, String> {
  let profile_manager = ProfileManager::instance();
  let profiles_dir = profile_manager.get_profiles_dir();

  let profile_uuid = uuid::Uuid::parse_str(&profile_id).map_err(|e| e.to_string())?;
  let profiles = profile_manager.list_profiles().map_err(|e| e.to_string())?;
  let profile = profiles
    .iter()
    .find(|p| p.id == profile_uuid)
    .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

  let profile_data_dir = profile.get_profile_data_path(&profiles_dir);

  // Create temp zip file with profile name instead of UUID
  let temp_dir = std::env::temp_dir().join("foxia-profile-sync");
  std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
  let zip_filename = format!("{}.zip", profile.name);
  let zip_path = temp_dir.join(&zip_filename);

  log::info!("Creating zip file: {}", zip_filename);

  // Zip directory
  if let Err(e) = s3_transfer::zip_directory(&profile_data_dir, &zip_path) {
    log::error!("Failed to zip directory {:?}: {}", profile_data_dir, e);
    return Err(format!("Failed to zip profile: {}", e));
  }

  // Upload to S3
  let profile_url = s3_transfer::upload_profile_to_s3(&base_url, &session_id, &zip_path)
    .await
    .map_err(|e| {
      log::error!("S3 upload error: {}", e);
      format!("Failed to upload to S3: {}", e)
    })?;

  // Clean up zip
  let _ = std::fs::remove_file(&zip_path);

  log::info!("Upload successful! Profile URL: {}", profile_url);

  Ok(profile_url)
}

#[tauri::command]
pub async fn download_profile_from_odoo_s3(
  app_handle: tauri::AppHandle,
  profile_id: String,
  profile_url: String,
) -> Result<(), String> {
  let profile_manager = ProfileManager::instance();
  let profiles_dir = profile_manager.get_profiles_dir();

  let profile_uuid = uuid::Uuid::parse_str(&profile_id).map_err(|e| e.to_string())?;
  let profiles = profile_manager.list_profiles().map_err(|e| e.to_string())?;
  let profile = profiles
    .iter()
    .find(|p| p.id == profile_uuid)
    .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

  let profile_data_dir = profile.get_profile_data_path(&profiles_dir);

  // Download with progress tracking
  let profile_id_clone = profile_id.clone();
  let profile_name = profile.name.clone();

  s3_transfer::download_and_extract_profile_with_progress(
    &profile_url,
    &profile_data_dir,
    move |downloaded, total| {
      let percentage = if total > 0 {
        (downloaded as f64 / total as f64 * 100.0) as u32
      } else {
        0
      };

      log::info!(
        "Download progress: {}/{} bytes ({}%)",
        downloaded,
        total,
        percentage
      );

      let _ = app_handle.emit(
        "download-progress",
        serde_json::json!({
          "profile_id": profile_id_clone,
          "profile_name": profile_name,
          "downloaded": downloaded,
          "total": total,
          "percentage": percentage
        }),
      );
    },
  )
  .await
  .map_err(|e| e.to_string())?;

  // Emit event to notify frontend that profiles changed
  let _ = events::emit_empty("profiles-changed");
  log::info!("Profile downloaded successfully, emitted profiles-changed event");

  Ok(())
}

#[tauri::command]
pub async fn is_odoo_logged_in() -> bool {
  ODOO_CLIENT.lock().await.is_some()
}

#[tauri::command]
pub async fn odoo_logout() {
  let mut odoo_client = ODOO_CLIENT.lock().await;
  *odoo_client = None;
}

#[tauri::command]
pub async fn list_odoo_profiles(offset: u32, limit: u32) -> Result<OdooListResult, String> {
  let odoo_client = ODOO_CLIENT.lock().await;
  if let Some(client) = odoo_client.as_ref() {
    client
      .list_profiles(offset, limit)
      .await
      .map_err(|e| e.to_string())
  } else {
    Err("Not logged in to Odoo".to_string())
  }
}

#[tauri::command]
pub async fn create_odoo_profile(profile: OdooProfile) -> Result<serde_json::Value, String> {
  let odoo_client = ODOO_CLIENT.lock().await;
  if let Some(client) = odoo_client.as_ref() {
    client
      .create_profile(profile)
      .await
      .map_err(|e| e.to_string())
  } else {
    Err("Not logged in to Odoo".to_string())
  }
}

#[tauri::command]
pub async fn update_odoo_profile(profile: OdooProfile) -> Result<serde_json::Value, String> {
  let odoo_client = ODOO_CLIENT.lock().await;
  if let Some(client) = odoo_client.as_ref() {
    client
      .update_profile(profile)
      .await
      .map_err(|e| e.to_string())
  } else {
    Err("Not logged in to Odoo".to_string())
  }
}

#[tauri::command]
pub async fn delete_odoo_profile(id: serde_json::Value) -> Result<bool, String> {
  let odoo_client = ODOO_CLIENT.lock().await;
  if let Some(client) = odoo_client.as_ref() {
    client.delete_profile(id).await.map_err(|e| e.to_string())
  } else {
    Err("Not logged in to Odoo".to_string())
  }
}
