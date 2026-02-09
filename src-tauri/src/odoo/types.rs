use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OdooProxy {
  pub giaothuc: String,
  pub ip: String,
  pub port: serde_json::Value,
  pub tendangnhap: Option<String>,
  pub matkhau: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OdooProfile {
  pub id: serde_json::Value,
  pub name: String,
  #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
  pub user_agent: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timezone: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub language: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub platform: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub proxy_ids: Option<Vec<OdooProxy>>,
  #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none")]
  pub profile_url: Option<serde_json::Value>,
  #[serde(rename = "localPath", skip_serializing_if = "Option::is_none")]
  pub local_path: Option<serde_json::Value>,
  #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
  pub created_at: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub create_date: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub username: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub password: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub browser: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OdooResponse<T> {
  pub jsonrpc: String,
  pub id: Option<serde_json::Value>,
  pub result: Option<T>,
  pub error: Option<OdooError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OdooError {
  pub code: i32,
  pub message: String,
  pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OdooLoginResult {
  pub uid: i32,
  pub name: String,
  pub session_id: Option<String>,
  // Optional fields returned by some Odoo versions/custom modules
  pub db: Option<String>,
  pub login: Option<String>,
  pub is_quanlytainguyen: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OdooListResult {
  pub items: Vec<OdooProfile>,
  pub total_count: i32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct OdooParams<T> {
  pub params: T,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct OdooListParams {
  pub domain: Vec<Vec<serde_json::Value>>,
  pub context2: serde_json::Value,
  pub offset: u32,
  pub limit: u32,
  pub order: String,
}
