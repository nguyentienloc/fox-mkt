use reqwest::multipart;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::ZipWriter;

#[derive(Debug, Deserialize)]
pub struct UploadResponse {
  pub data: UploadResponseData,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UploadResponseData {
  #[allow(dead_code)]
  pub success: bool,
  #[allow(dead_code)]
  pub message: Option<String>,
  #[serde(rename = "profileUrl")]
  pub profile_url: Option<String>,
  pub url: Option<String>,
}

/// Zip a directory recursively
pub fn zip_directory(
  src_dir: &Path,
  dst_file: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let file = File::create(dst_file)?;
  let mut zip = ZipWriter::new(file);
  let options = FileOptions::default()
    .compression_method(zip::CompressionMethod::Deflated)
    .unix_permissions(0o755);

  fn walk_dir(
    dir: &Path,
    base: &Path,
    zip: &mut ZipWriter<File>,
    options: FileOptions<()>,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      let name = path.strip_prefix(base)?;

      if path.is_file() {
        log::debug!("Adding file to zip: {:?}", name);
        zip.start_file(name.to_string_lossy(), options)?;
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
      } else if path.is_dir() {
        log::debug!("Adding directory to zip: {:?}", name);
        zip.add_directory(name.to_string_lossy(), options)?;
        walk_dir(&path, base, zip, options)?;
      }
    }
    Ok(())
  }

  walk_dir(src_dir, src_dir, &mut zip, options)?;

  zip.finish()?;
  Ok(())
}

/// Upload profile zip to S3 proxy
pub async fn upload_profile_to_s3(
  base_url: &str,
  session_id: &str,
  zip_path: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let client = reqwest::Client::new();

  let file_content = fs::read(zip_path)?;
  let file_size = file_content.len();
  let filename = zip_path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("profile.zip")
    .to_string();

  let part = multipart::Part::bytes(file_content)
    .file_name(filename.clone())
    .mime_str("application/zip")?;

  let form = multipart::Form::new().part("file", part);

  let upload_url = format!(
    "https://backend-analytics.soly.vn/foxia/upload-profile-v2?domain={}&sessionId={}",
    urlencoding::encode(base_url),
    urlencoding::encode(session_id)
  );

  // Log curl command for debugging
  log::info!("=== S3 Upload Request ===");
  log::info!("URL: {}", upload_url);
  log::info!("File: {}", filename);
  log::info!(
    "Size: {} bytes ({:.2} MB)",
    file_size,
    file_size as f64 / 1024.0 / 1024.0
  );
  log::info!("Curl command:");
  log::info!(
    "curl -X POST '{}' \\\n  -F 'file=@{}'",
    upload_url,
    zip_path.display()
  );
  log::info!("========================");

  let response = client.post(&upload_url).multipart(form).send().await?;

  let status = response.status();
  log::info!("Response status: {}", status);

  if !status.is_success() {
    let error_body = response.text().await.unwrap_or_default();
    log::error!(
      "S3 upload failed with status: {}, body: {}",
      status,
      error_body
    );
    return Err(format!("S3 upload failed with status: {}", status).into());
  }

  let response_text = response.text().await?;
  log::info!("Response body: {}", response_text);

  let res: UploadResponse = serde_json::from_str(&response_text)
    .map_err(|e| format!("Failed to parse response: {}. Body: {}", e, response_text))?;

  let profile_url = res
    .data
    .profile_url
    .or(res.data.url)
    .ok_or("Upload response missing profile URL")?;

  log::info!("Upload successful! Profile URL: {}", profile_url);

  Ok(profile_url)
}

/// Download and extract profile from S3 with progress tracking
#[allow(dead_code)]
pub async fn download_and_extract_profile(
  profile_url: &str,
  dest_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let client = reqwest::Client::new();
  let response = client.get(profile_url).send().await?;

  if !response.status().is_success() {
    return Err(format!("Failed to download profile: {}", response.status()).into());
  }

  let total_size = response.content_length().unwrap_or(0);
  log::info!(
    "Downloading profile: {} bytes ({:.2} MB)",
    total_size,
    total_size as f64 / 1024.0 / 1024.0
  );

  let content = response.bytes().await?;
  let downloaded_size = content.len();

  log::info!(
    "Download complete: {} bytes ({:.2} MB)",
    downloaded_size,
    downloaded_size as f64 / 1024.0 / 1024.0
  );

  let reader = std::io::Cursor::new(content);
  let mut archive = zip::ZipArchive::new(reader)?;

  if !dest_dir.exists() {
    fs::create_dir_all(dest_dir)?;
  }

  log::info!("Extracting {} files...", archive.len());

  let total_files = archive.len();
  for i in 0..total_files {
    let mut file = archive.by_index(i)?;
    let outpath = match file.enclosed_name() {
      Some(path) => dest_dir.join(path),
      None => continue,
    };

    if file.name().ends_with('/') {
      fs::create_dir_all(&outpath)?;
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          fs::create_dir_all(p)?;
        }
      }
      let mut outfile = fs::File::create(&outpath)?;
      std::io::copy(&mut file, &mut outfile)?;
    }

    let progress = ((i + 1) as f32 / total_files as f32 * 100.0) as u32;
    if i % 10 == 0 || i == total_files - 1 {
      log::debug!(
        "Extraction progress: {}% ({}/{})",
        progress,
        i + 1,
        total_files
      );
    }
  }

  log::info!("Extraction complete!");

  Ok(())
}

/// Download profile with progress callback
pub async fn download_and_extract_profile_with_progress<F>(
  profile_url: &str,
  dest_dir: &Path,
  progress_callback: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
  F: Fn(u64, u64) + Send + 'static,
{
  use futures_util::StreamExt;

  let client = reqwest::Client::new();
  let response = client.get(profile_url).send().await?;

  if !response.status().is_success() {
    return Err(format!("Failed to download profile: {}", response.status()).into());
  }

  let total_size = response.content_length().unwrap_or(0);
  log::info!(
    "Downloading profile: {} bytes ({:.2} MB)",
    total_size,
    total_size as f64 / 1024.0 / 1024.0
  );

  let mut downloaded: u64 = 0;
  let mut stream = response.bytes_stream();
  let mut buffer = Vec::new();

  // Report every 256KB to get smoother progress updates
  let chunk_report_size = 256 * 1024; // 256KB
  let mut last_reported = 0;

  progress_callback(0, total_size);

  while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    buffer.extend_from_slice(&chunk);
    downloaded += chunk.len() as u64;

    // Report progress every 256KB or on completion
    if downloaded - last_reported >= chunk_report_size || downloaded >= total_size {
      progress_callback(downloaded, total_size);
      last_reported = downloaded;
      log::debug!(
        "Download progress: {:.2} MB / {:.2} MB ({:.1}%)",
        downloaded as f64 / 1024.0 / 1024.0,
        total_size as f64 / 1024.0 / 1024.0,
        (downloaded as f64 / total_size as f64 * 100.0)
      );
    }
  }

  // Ensure final progress is reported
  if downloaded != last_reported {
    progress_callback(downloaded, total_size);
  }

  log::info!(
    "Download complete: {} bytes ({:.2} MB)",
    downloaded,
    downloaded as f64 / 1024.0 / 1024.0
  );

  let reader = std::io::Cursor::new(buffer);
  let mut archive = zip::ZipArchive::new(reader)?;

  if !dest_dir.exists() {
    fs::create_dir_all(dest_dir)?;
  }

  log::info!("Extracting {} files...", archive.len());

  for i in 0..archive.len() {
    let mut file = archive.by_index(i)?;
    let outpath = match file.enclosed_name() {
      Some(path) => dest_dir.join(path),
      None => continue,
    };

    if file.name().ends_with('/') {
      fs::create_dir_all(&outpath)?;
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          fs::create_dir_all(p)?;
        }
      }
      let mut outfile = fs::File::create(&outpath)?;
      std::io::copy(&mut file, &mut outfile)?;
    }
  }

  log::info!("Extraction complete!");

  Ok(())
}
