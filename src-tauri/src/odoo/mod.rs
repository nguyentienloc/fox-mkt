pub mod client;
pub mod commands;
pub mod types;

pub use client::OdooClient;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static ODOO_CLIENT: Lazy<Arc<Mutex<Option<OdooClient>>>> =
  Lazy::new(|| Arc::new(Mutex::new(None)));

#[allow(dead_code)]
pub async fn get_odoo_client() -> Option<OdooClient> {
  let _client = ODOO_CLIENT.lock().await;
  // We can't easily clone OdooClient because of the Jar and reqwest Client
  // but we can maybe return a reference or use a better structure.
  // For now, let's assume we'll just initialize it when needed.
  None
}
