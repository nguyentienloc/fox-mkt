use crate::odoo::types::*;
use reqwest::{cookie::Jar, Client};
use serde_json::json;
use std::sync::Arc;

pub struct OdooClient {
  client: Client,
  #[allow(dead_code)]
  jar: Arc<Jar>,
  base_url: String,
}

impl OdooClient {
  pub fn new(base_url: String) -> Self {
    let jar = Arc::new(Jar::default());
    let client = Client::builder()
      .cookie_store(true)
      .cookie_provider(jar.clone())
      .build()
      .unwrap_or_else(|_| Client::new());

    Self {
      client,
      jar,
      base_url,
    }
  }

  pub async fn login(
    &self,
    login: String,
    password: String,
  ) -> Result<OdooLoginResult, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/res_users/login", self.base_url);

    let body = json!({
        "params": {
            "login": login,
            "password": password,
        }
    });

    let response = self.client.post(&url)
            .header("accept", "application/json")
            .header("content-type", "application/json;charset=UTF-8")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) foxia_mkt/0.0.38 Chrome/138.0.7204.251 Electron/37.10.3 Safari/537.36")
            .json(&body)
            .send()
            .await?;

    let status = response.status();

    // Extract session_id from Set-Cookie header
    let mut session_id_from_cookie: Option<String> = None;
    if let Some(cookie_header) = response.headers().get("set-cookie") {
      if let Ok(cookie_str) = cookie_header.to_str() {
        log::info!("Set-Cookie header: {}", cookie_str);
        // Parse session_id from cookie string like: "session_id=abc123; Path=/; HttpOnly"
        if let Some(start) = cookie_str.find("session_id=") {
          let session_part = &cookie_str[start..];
          if let Some(end) = session_part.find(';') {
            session_id_from_cookie = Some(session_part[11..end].to_string());
          } else {
            session_id_from_cookie = Some(session_part[11..].to_string());
          }
          log::info!(
            "Extracted session_id from cookie: {:?}",
            session_id_from_cookie
          );
        }
      }
    }

    let text = response.text().await?;

    log::info!("Odoo login response status: {}, body: {}", status, text);

    let res: OdooResponse<OdooLoginResult> = match serde_json::from_str(&text) {
      Ok(r) => r,
      Err(e) => {
        return Err(format!("Failed to parse Odoo response: {}. Body: {}", e, text).into());
      }
    };

    if let Some(error) = res.error {
      return Err(format!("Odoo login failed: {}", error.message).into());
    }

    if let Some(mut result) = res.result {
      // Prioritize session_id from cookie if not in body
      if result.session_id.is_none() && session_id_from_cookie.is_some() {
        log::info!("Using session_id from cookie since not in response body");
        result.session_id = session_id_from_cookie;
      }

      if let Some(sid) = &result.session_id {
        log::info!("Odoo session_id found: {}", sid);
      } else {
        log::warn!("No session_id found in response body or cookie!");
      }
      Ok(result)
    } else {
      Err("Odoo login returned no result".into())
    }
  }

  pub async fn list_profiles(
    &self,
    offset: u32,
    limit: u32,
  ) -> Result<OdooListResult, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/hosotainguyen/list", self.base_url);

    let body = json!({
        "params": {
            "domain": [],
            "context2": {},
            "offset": offset,
            "limit": limit,
            "order": "id desc",
        }
    });

    let response = self.client.post(&url)
            .header("accept", "application/json")
            .header("content-type", "application/json;charset=UTF-8")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) foxia_mkt/0.0.38 Chrome/138.0.7204.251 Electron/37.10.3 Safari/537.36")
            .json(&body)
            .send()
            .await?;

    let status = response.status();
    let text = response.text().await?;

    // Cần in ra Terminal để debug nếu parse lỗi
    log::info!("Odoo list_profiles response status: {}", status);

    let res: OdooResponse<OdooListResult> = match serde_json::from_str(&text) {
      Ok(r) => r,
      Err(e) => {
        log::error!("Failed to parse Odoo list response: {}. Body: {}", e, text);
        return Err(format!("Failed to parse Odoo list response: {}", e).into());
      }
    };

    if let Some(error) = res.error {
      return Err(format!("Odoo list profiles failed: {}", error.message).into());
    }

    if let Some(result) = res.result {
      Ok(result)
    } else {
      Err("Odoo list profiles returned no result".into())
    }
  }

  pub async fn create_profile(
    &self,
    profile: OdooProfile,
  ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/hosotainguyen/create", self.base_url);

    let body = json!({
        "params": {
            "params": profile,
            "context2": {},
        }
    });

    log::info!("Odoo create_profile request to: {}, body: {}", url, body);

    let response = self.client.post(&url).json(&body).send().await?;
    let text = response.text().await?;
    log::info!("Odoo create_profile response: {}", text);

    let res: OdooResponse<serde_json::Value> = serde_json::from_str(&text)?;

    if let Some(error) = res.error {
      return Err(format!("Odoo create profile failed: {}", error.message).into());
    }

    Ok(res.result.unwrap_or(serde_json::Value::Null))
  }

  pub async fn update_profile(
    &self,
    profile: OdooProfile,
  ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/hosotainguyen/write", self.base_url);

    let body = json!({
        "params": {
            "params": profile,
            "domain": [["id", "=", profile.id]],
            "context2": {},
        }
    });

    log::info!("Odoo update_profile request to: {}, body: {}", url, body);

    let response = self.client.post(&url).json(&body).send().await?;
    let text = response.text().await?;
    log::info!("Odoo update_profile response: {}", text);

    let res: OdooResponse<serde_json::Value> = serde_json::from_str(&text)?;

    if let Some(error) = res.error {
      return Err(format!("Odoo update profile failed: {}", error.message).into());
    }

    Ok(res.result.unwrap_or(serde_json::Value::Null))
  }

  pub async fn delete_profile(
    &self,
    id: serde_json::Value,
  ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/hosotainguyen/unlink", self.base_url);

    let body = json!({
        "params": {
            "domain": [["id", "=", id]],
            "context2": {},
            "offset": 0,
            "limit": 1,
            "order": "id asc",
        }
    });

    log::info!("Odoo delete_profile request to: {}, body: {}", url, body);

    let response = self.client.post(&url).json(&body).send().await?;
    let status = response.status();
    let text = response.text().await?;

    log::info!("Odoo delete_profile response status: {}, body: {}", status, text);

    // Odoo có thể trả response không đúng format OdooResponse<bool>
    match serde_json::from_str::<OdooResponse<bool>>(&text) {
      Ok(res) => {
        if let Some(error) = res.error {
          return Err(format!("Odoo delete profile failed: {}", error.message).into());
        }
        Ok(res.result.unwrap_or(false))
      }
      Err(e) => {
        log::warn!("Could not parse delete response as OdooResponse<bool>: {}. Trying OdooResponse<Value>...", e);
        // Fallback: thử parse dạng khác, nếu status OK thì coi như thành công
        match serde_json::from_str::<OdooResponse<serde_json::Value>>(&text) {
          Ok(res) => {
            if let Some(error) = res.error {
              return Err(format!("Odoo delete profile failed: {}", error.message).into());
            }
            log::info!("Delete profile succeeded (parsed as Value): {:?}", res.result);
            Ok(true)
          }
          Err(_) => {
            if status.is_success() {
              log::warn!("Could not parse response but HTTP status is OK, assuming success");
              Ok(true)
            } else {
              Err(format!("Failed to parse Odoo delete response: {}. Body: {}", e, text).into())
            }
          }
        }
      }
    }
  }
}
