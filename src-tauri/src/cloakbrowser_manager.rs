use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloakBrowserConfig {
  #[serde(default)]
  pub seed: Option<u32>,
  #[serde(default)]
  pub platform: Option<String>,
  #[serde(default)]
  pub timezone: Option<String>,
  #[serde(default)]
  pub locale: Option<String>,
  #[serde(default)]
  pub user_agent: Option<String>,
}

impl CloakBrowserConfig {
  pub fn generate_seed() -> u32 {
    rand::rng().random_range(10000..99999)
  }

  pub fn to_launch_args(&self) -> Vec<String> {
    let mut args = Vec::new();

    let seed = self.seed.unwrap_or_else(Self::generate_seed);
    args.push(format!("--fingerprint={seed}"));

    if let Some(ref platform) = self.platform {
      args.push(format!("--fingerprint-platform={platform}"));
    } else {
      args.push("--fingerprint-platform=windows".to_string());
    }

    if let Some(ref tz) = self.timezone {
      args.push(format!("--fingerprint-timezone={tz}"));
    }

    if let Some(ref locale) = self.locale {
      args.push(format!("--fingerprint-locale={locale}"));
    }

    if let Some(ref ua) = self.user_agent {
      args.push(format!("--user-agent={ua}"));
    }

    args
  }
}
