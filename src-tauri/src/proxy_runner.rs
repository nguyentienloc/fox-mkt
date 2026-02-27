use crate::proxy_storage::{
  delete_proxy_config, generate_proxy_id, get_proxy_config, is_process_running, list_proxy_configs,
  save_proxy_config, ProxyConfig,
};
use std::process::Stdio;
lazy_static::lazy_static! {
  static ref PROXY_PROCESSES: std::sync::Mutex<std::collections::HashMap<String, u32>> =
    std::sync::Mutex::new(std::collections::HashMap::new());
}

pub async fn start_proxy_process(
  upstream_url: Option<String>,
  port: Option<u16>,
) -> Result<ProxyConfig, Box<dyn std::error::Error>> {
  start_proxy_process_with_profile(upstream_url, port, None).await
}

pub async fn start_proxy_process_with_profile(
  upstream_url: Option<String>,
  port: Option<u16>,
  profile_id: Option<String>,
) -> Result<ProxyConfig, Box<dyn std::error::Error>> {
  let id = generate_proxy_id();
  let upstream = upstream_url.unwrap_or_else(|| "DIRECT".to_string());

  // Dùng port=0 để process con tự bind vào port available
  // Tránh race condition: pre-allocate rồi drop listener có thể fail trên Windows
  // khi OS chưa release port kịp trước khi process con bind lại
  let local_port = port.unwrap_or(0);

  let config =
    ProxyConfig::new(id.clone(), upstream, Some(local_port)).with_profile_id(profile_id.clone());
  save_proxy_config(&config)?;

  // Log profile_id for debugging
  if let Some(ref pid) = profile_id {
    log::info!("Saved proxy config {} with profile_id: {}", id, pid);
  } else {
    log::info!("Saved proxy config {} without profile_id", id);
  }

  // Spawn proxy worker process in the background using std::process::Command
  // This ensures proper process detachment on Unix systems
  let exe = std::env::current_exe()?;
  log::info!("Starting proxy worker from binary: {:?}", exe);

  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    use std::process::Command as StdCommand;

    let mut cmd = StdCommand::new(&exe);
    cmd.arg("proxy-worker");
    cmd.arg("start");
    cmd.arg("--id");
    cmd.arg(&id);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());

    // Always log to file for diagnostics (both debug and release builds)
    let log_path = std::path::PathBuf::from("/tmp").join(format!("foxia-proxy-{}.log", id));
    if let Ok(file) = std::fs::File::create(&log_path) {
      log::info!("Proxy worker stderr will be logged to: {:?}", log_path);
      cmd.stderr(Stdio::from(file));
    } else {
      cmd.stderr(Stdio::null());
    }

    // Properly detach the process on Unix by creating a new session
    unsafe {
      cmd.pre_exec(|| {
        // Create a new process group so the process survives parent exit
        libc::setsid();

        // Set high priority so the proxy is killed last under resource pressure
        // Negative nice value = higher priority. Try -10, fall back to -5 if it fails.
        if libc::setpriority(libc::PRIO_PROCESS, 0, -10) != 0 {
          let _ = libc::setpriority(libc::PRIO_PROCESS, 0, -5);
        }

        Ok(())
      });
    }

    // Spawn detached process
    let child = cmd.spawn()?;
    let pid = child.id();

    // Store PID
    {
      let mut processes = PROXY_PROCESSES.lock().unwrap();
      processes.insert(id.clone(), pid);
    }

    // Update config with PID
    let mut config_with_pid = config.clone();
    config_with_pid.pid = Some(pid);
    save_proxy_config(&config_with_pid)?;

    // Don't wait for the child - it's detached
    drop(child);
  }

  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    use std::process::Command as StdCommand;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
      OpenProcess, SetPriorityClass, ABOVE_NORMAL_PRIORITY_CLASS, PROCESS_SET_INFORMATION,
    };

    let mut cmd = StdCommand::new(&exe);
    cmd.arg("proxy-worker");
    cmd.arg("start");
    cmd.arg("--id");
    cmd.arg(&id);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());

    // Log stderr ra file để debug (quan trọng trên Windows vì không có terminal)
    let log_path = std::env::temp_dir().join(format!("foxia-proxy-{}.log", id));
    if let Ok(file) = std::fs::File::create(&log_path) {
      log::info!("Proxy worker stderr will be logged to: {:?}", log_path);
      cmd.stderr(Stdio::from(file));
    } else {
      cmd.stderr(Stdio::null());
    }

    // DETACHED_PROCESS: ngắt kế thừa stdout/stdin console handles từ parent
    // Quan trọng: sidecar foxia-proxy chạy bởi Tauri có stdout được pipe để đọc JSON output.
    // Nếu child kế thừa stdout pipe đó, Tauri sẽ block đợi EOF mãi mãi (deadlock).
    // DETACHED_PROCESS giải quyết vấn đề này bằng cách không kế thừa console của parent.
    // CREATE_NEW_PROCESS_GROUP: tạo process group riêng để process sống độc lập khi parent exit.
    // CREATE_NO_WINDOW: không mở console window visible trên Windows.
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);

    let child = cmd.spawn()?;
    let pid = child.id();

    // Set high priority so the proxy is killed last under resource pressure
    unsafe {
      if let Ok(handle) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) {
        let _ = SetPriorityClass(handle, ABOVE_NORMAL_PRIORITY_CLASS);
        let _ = CloseHandle(handle);
      }
    }

    // Store PID
    {
      let mut processes = PROXY_PROCESSES.lock().unwrap();
      processes.insert(id.clone(), pid);
    }

    // Update config with PID
    let mut config_with_pid = config.clone();
    config_with_pid.pid = Some(pid);
    save_proxy_config(&config_with_pid)?;

    drop(child);
  }

  // Give the process a moment to start up before checking
  // Windows cần nhiều thời gian hơn do overhead process creation và antivirus scan
  #[cfg(windows)]
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
  #[cfg(not(windows))]
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Wait for the worker to bind và update config với local_url
  let mut attempts = 0;
  // Windows cần timeout dài hơn (15s) để handle slow startup
  #[cfg(windows)]
  let max_attempts = 150;
  #[cfg(not(windows))]
  let max_attempts = 80; // 8s cho unix

  loop {
    // Use shorter sleep for faster startup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    if let Some(updated_config) = get_proxy_config(&id) {
      // Check if local_url is set (worker đã bind port và update config)
      if let Some(ref local_url) = updated_config.local_url {
        if !local_url.is_empty() {
          if let Some(port) = updated_config.local_port {
            // Verify port thực sự đang listen
            match tokio::time::timeout(
              tokio::time::Duration::from_millis(200),
              tokio::net::TcpStream::connect(("127.0.0.1", port)),
            )
            .await
            {
              Ok(Ok(_stream)) => {
                // Port đang listen và chấp nhận kết nối
                return Ok(updated_config);
              }
              Ok(Err(_)) | Err(_) => {
                // Port chưa ready, tiếp tục chờ
              }
            }
          }
        }
      }
    }

    attempts += 1;
    if attempts >= max_attempts {
      // Try to get the config one more time for better error message
      if let Some(config) = get_proxy_config(&id) {
        // Check if process is still running
        let process_running = config.pid.map(is_process_running).unwrap_or(false);

        // Try to read the last few lines of the log file for diagnostics
        #[cfg(windows)]
        let log_path = std::env::temp_dir().join(format!("foxia-proxy-{}.log", id));
        #[cfg(not(windows))]
        let log_path = std::path::PathBuf::from("/tmp").join(format!("foxia-proxy-{}.log", id));

        let log_content = if log_path.exists() {
          std::fs::read_to_string(&log_path)
            .map(|s| {
              let lines: Vec<&str> = s.lines().collect();
              let last_lines = lines
                .iter()
                .rev()
                .take(10)
                .rev()
                .cloned()
                .collect::<Vec<&str>>();
              format!(
                "\n--- Last 10 lines of worker log ---\n{}\n--- End of log ---",
                last_lines.join("\n")
              )
            })
            .unwrap_or_else(|e| format!(" (failed to read log file: {})", e))
        } else {
          format!(" (log file not found at {:?})", log_path)
        };

        return Err(
          format!(
            "Proxy worker failed to start in time after {} attempts. Config: id={}, local_url={:?}, local_port={:?}, pid={:?}, process_running={}{}",
            attempts, config.id, config.local_url, config.local_port, config.pid, process_running, log_content
          )
          .into(),
        );
      }
      return Err(
        format!(
          "Proxy worker failed to start in time. Config not found for id: {}",
          id
        )
        .into(),
      );
    }
  }
}

pub async fn stop_proxy_process(id: &str) -> Result<bool, Box<dyn std::error::Error>> {
  let config = get_proxy_config(id);

  if let Some(config) = config {
    if let Some(pid) = config.pid {
      // Kill the process
      #[cfg(unix)]
      {
        use std::process::Command;
        let _ = Command::new("kill")
          .arg("-TERM")
          .arg(pid.to_string())
          .output();
      }
      #[cfg(windows)]
      {
        use std::process::Command;
        let _ = Command::new("taskkill")
          .args(["/F", "/PID", &pid.to_string()])
          .output();
      }

      // Wait a bit for the process to exit
      tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

      // Remove from tracking
      {
        let mut processes = PROXY_PROCESSES.lock().unwrap();
        processes.remove(id);
      }

      // Delete the config file
      delete_proxy_config(id);
      return Ok(true);
    }
  }

  Ok(false)
}

pub async fn stop_all_proxy_processes() -> Result<(), Box<dyn std::error::Error>> {
  let configs = list_proxy_configs();
  for config in configs {
    let _ = stop_proxy_process(&config.id).await;
  }
  Ok(())
}
