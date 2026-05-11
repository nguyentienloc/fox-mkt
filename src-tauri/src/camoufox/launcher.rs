//! Camoufox browser process launcher.
//!
//! Provides functionality to launch Camoufox browser instances with fingerprint injection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::{Child, Command};

use crate::camoufox::config::{CamoufoxConfigBuilder, CamoufoxLaunchConfig, ProxyConfig};
use crate::camoufox::fingerprint::types::{Fingerprint, ScreenConstraints};

/// Camoufox launcher for creating browser instances.
pub struct CamoufoxLauncher {
  executable_path: PathBuf,
}

/// Running Camoufox process.
#[derive(Debug)]
pub struct CamoufoxProcess {
  child: Child,
}

impl CamoufoxProcess {
  /// Return the operating system process id.
  pub fn id(&self) -> Option<u32> {
    self.child.id()
  }

  /// Stop the browser process.
  pub async fn close(mut self) -> std::io::Result<()> {
    match self.child.start_kill() {
      Ok(()) => {
        let _ = self.child.wait().await;
        Ok(())
      }
      Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
      Err(error) => Err(error),
    }
  }
}

/// Error type for launcher operations.
#[derive(Debug, thiserror::Error)]
pub enum LauncherError {
  #[error("Configuration error: {0}")]
  Config(#[from] crate::camoufox::config::ConfigError),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Camoufox executable not found at: {0}")]
  ExecutableNotFound(PathBuf),

  #[error("Failed to generate environment variables: {0}")]
  EnvVars(#[from] serde_json::Error),
}

/// Options for launching Camoufox.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
  /// Operating system to spoof: "windows", "macos", "linux"
  pub os: Option<String>,
  /// Block all images
  pub block_images: bool,
  /// Block WebRTC entirely
  pub block_webrtc: bool,
  /// Block WebGL (not recommended unless necessary)
  pub block_webgl: bool,
  /// Screen dimension constraints
  pub screen: Option<ScreenConstraints>,
  /// Fixed window size [width, height]
  pub window: Option<(u32, u32)>,
  /// Custom fingerprint (if not provided, one will be generated)
  pub fingerprint: Option<Fingerprint>,
  /// Run in headless mode
  pub headless: bool,
  /// Custom fonts to load
  pub fonts: Option<Vec<String>>,
  /// Only use custom fonts (disable OS fonts)
  pub custom_fonts_only: bool,
  /// Firefox user preferences
  pub firefox_user_prefs: Option<HashMap<String, serde_json::Value>>,
  /// Proxy configuration
  pub proxy: Option<ProxyConfig>,
  /// Additional browser arguments
  pub args: Option<Vec<String>>,
  /// Additional environment variables
  pub env: Option<HashMap<String, String>>,
  /// Profile/user data directory
  pub user_data_dir: Option<PathBuf>,
  /// Enable debug output
  pub debug: bool,
}

impl CamoufoxLauncher {
  /// Create a new Camoufox launcher.
  pub async fn new(executable_path: impl AsRef<Path>) -> Result<Self, LauncherError> {
    let executable_path = executable_path.as_ref().to_path_buf();

    if !executable_path.exists() {
      return Err(LauncherError::ExecutableNotFound(executable_path));
    }

    Ok(Self { executable_path })
  }

  /// Launch a new Camoufox browser process.
  pub async fn launch(&self, options: LaunchOptions) -> Result<CamoufoxProcess, LauncherError> {
    let user_data_dir = options.user_data_dir.clone();
    self.launch_process(user_data_dir.as_deref(), options).await
  }

  /// Launch a persistent browser process.
  pub async fn launch_persistent_context(
    &self,
    user_data_dir: impl AsRef<Path>,
    options: LaunchOptions,
  ) -> Result<CamoufoxProcess, LauncherError> {
    self
      .launch_process(Some(user_data_dir.as_ref()), options)
      .await
  }

  async fn launch_process(
    &self,
    user_data_dir: Option<&Path>,
    options: LaunchOptions,
  ) -> Result<CamoufoxProcess, LauncherError> {
    let config = self.build_config(&options)?;

    if options.debug {
      log::debug!("Camoufox config: {:?}", config.fingerprint_config);
    }

    let env_vars = config.get_env_vars()?;
    let mut args = options.args.clone().unwrap_or_default();

    if let Some(user_data_dir) = user_data_dir {
      args.push("-profile".to_string());
      args.push(user_data_dir.to_string_lossy().to_string());
    }

    if options.headless {
      args.push("--headless".to_string());
    }

    let mut env = options.env.clone().unwrap_or_default();
    for (key, value) in env_vars {
      env.insert(key, value);
    }

    if cfg!(target_os = "linux") {
      if let Some(fontconfig_path) =
        crate::camoufox::env_vars::get_fontconfig_env(&config.target_os, &self.executable_path)
      {
        env.insert("FONTCONFIG_PATH".to_string(), fontconfig_path);
      }
    }

    if let Some(proxy) = &config.proxy {
      let proxy_arg = self.proxy_arg(proxy);
      if !proxy_arg.is_empty() {
        args.push(proxy_arg);
      }
    }

    let mut command = Command::new(&self.executable_path);
    command
      .args(&args)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null());

    for (key, value) in env {
      command.env(key, value);
    }

    Ok(CamoufoxProcess {
      child: command.spawn()?,
    })
  }

  /// Build Camoufox configuration from launch options.
  fn build_config(&self, options: &LaunchOptions) -> Result<CamoufoxLaunchConfig, LauncherError> {
    let mut builder = CamoufoxConfigBuilder::new();

    if let Some(os) = &options.os {
      builder = builder.operating_system(os);
    }

    if let Some(screen) = &options.screen {
      builder = builder.screen_constraints(screen.clone());
    }

    if let Some(fingerprint) = &options.fingerprint {
      builder = builder.fingerprint(fingerprint.clone());
    }

    builder = builder.block_images(options.block_images);
    builder = builder.block_webrtc(options.block_webrtc);
    builder = builder.block_webgl(options.block_webgl);
    builder = builder.headless(options.headless);

    if let Some(fonts) = &options.fonts {
      builder = builder.custom_fonts(fonts.clone());
    }

    builder = builder.custom_fonts_only(options.custom_fonts_only);

    if let Some(proxy) = &options.proxy {
      builder = builder.proxy(proxy.clone());
    }

    if let Some(version) = crate::camoufox::config::get_firefox_version(&self.executable_path) {
      builder = builder.ff_version(version);
    }

    Ok(builder.build()?)
  }

  fn proxy_arg(&self, proxy: &ProxyConfig) -> String {
    let mut proxy_url = proxy.server.clone();

    if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
      if let Ok(mut parsed) = url::Url::parse(&proxy.server) {
        let _ = parsed.set_username(username);
        let _ = parsed.set_password(Some(password));
        proxy_url = parsed.to_string();
      }
    }

    format!("--proxy-server={proxy_url}")
  }

  /// Get the executable path.
  pub fn executable_path(&self) -> &Path {
    &self.executable_path
  }
}

/// Convenience function to launch Camoufox with default settings.
pub async fn launch_camoufox(
  executable_path: impl AsRef<Path>,
  options: LaunchOptions,
) -> Result<CamoufoxProcess, LauncherError> {
  let launcher = CamoufoxLauncher::new(executable_path).await?;
  launcher.launch(options).await
}

/// Convenience function to launch a persistent Camoufox process.
pub async fn launch_persistent_camoufox(
  executable_path: impl AsRef<Path>,
  user_data_dir: impl AsRef<Path>,
  options: LaunchOptions,
) -> Result<CamoufoxProcess, LauncherError> {
  let launcher = CamoufoxLauncher::new(executable_path).await?;
  launcher
    .launch_persistent_context(user_data_dir, options)
    .await
}
