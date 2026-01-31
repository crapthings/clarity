use serde::Deserialize;
use std::path::PathBuf;
use tokio::process::Command;
use log;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use std::time::Duration;

// Google Gemini API 响应结构
#[derive(Debug, Deserialize)]
struct GeminiGenerateContentResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<i64>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<i64>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<i64>,
}

// Google Gemini File API 响应结构
#[derive(Debug, Deserialize)]
struct GeminiFileUploadResponse {
    file: GeminiFile,
}

#[derive(Debug, Deserialize)]
struct GeminiFile {
    name: String,
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    state: String,
    // Additional optional fields that might be present
    #[serde(rename = "displayName")]
    #[serde(default)]
    display_name: Option<String>,
    #[serde(rename = "sizeBytes")]
    #[serde(default)]
    size_bytes: Option<String>,
    #[serde(rename = "createTime")]
    #[serde(default)]
    create_time: Option<String>,
    #[serde(rename = "updateTime")]
    #[serde(default)]
    update_time: Option<String>,
    #[serde(rename = "expirationTime")]
    #[serde(default)]
    expiration_time: Option<String>,
    #[serde(rename = "sha256Hash")]
    #[serde(default)]
    sha256_hash: Option<String>,
    #[serde(rename = "downloadUri")]
    #[serde(default)]
    download_uri: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GeminiFileGetResponse {
    file: GeminiFile,
}

// API 请求结果，包含响应内容和 token 使用情况
#[derive(Debug)]
pub struct ApiRequestResult {
    pub content: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub status_code: u16,
    pub duration_ms: u64,
}

// 从图片列表创建视频（使用 ffmpeg）
pub async fn create_video_from_images(
    image_paths: &[PathBuf],
    output_path: &PathBuf,
    fps: u32,
) -> Result<(), String> {
    if image_paths.is_empty() {
        return Err("No images to create video from".to_string());
    }

    // 检查 ffmpeg 是否可用
    // 在 macOS 上，尝试多个可能的路径
    let ffmpeg_paths = if cfg!(target_os = "macos") {
        vec!["ffmpeg", "/usr/local/bin/ffmpeg", "/opt/homebrew/bin/ffmpeg"]
    } else {
        vec!["ffmpeg"]
    };
    
    let mut ffmpeg_found = false;
    let mut ffmpeg_path = String::from("ffmpeg");
    
    for path in &ffmpeg_paths {
        let check = Command::new(path)
            .arg("-version")
            .output()
            .await;
        
        if check.is_ok() {
            ffmpeg_found = true;
            ffmpeg_path = path.to_string();
            log::info!("Found ffmpeg at: {}", path);
            break;
        }
    }
    
    if !ffmpeg_found {
        let error_msg = format!(
            "ffmpeg not found. Please install ffmpeg to create videos. Tried paths: {:?}",
            ffmpeg_paths
        );
        log::error!("{}", error_msg);
        return Err(error_msg);
    }

    // 创建临时文件列表
    let temp_list_path = output_path.parent()
        .ok_or("Invalid output path")?
        .join("ffmpeg_list.txt");

    // 写入文件列表（每张图片显示 1/fps 秒）
    let mut list_content = String::new();
    for path in image_paths {
        list_content.push_str(&format!("file '{}'\n", path.display()));
        list_content.push_str(&format!("duration {}\n", 1.0 / fps as f64));
    }
    // 最后一张图片需要重复一次（ffmpeg 要求）
    if let Some(last) = image_paths.last() {
        list_content.push_str(&format!("file '{}'\n", last.display()));
    }

    tokio::fs::write(&temp_list_path, list_content)
        .await
        .map_err(|e| format!("Failed to write file list: {}", e))?;

    // 使用 ffmpeg 创建视频
    log::info!("Running ffmpeg to create video from {} images", image_paths.len());
    let output = Command::new(&ffmpeg_path)
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&temp_list_path)
        // 降低分辨率以减少 token 消耗：640x360 对于屏幕活动分析已经足够
        // 如果需要更高质量，可以改为 960x540
        .arg("-vf")
        .arg("scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("fast")
        .arg("-crf")
        .arg("23")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-r")
        .arg(&fps.to_string())
        .arg("-y")
        .arg(output_path)
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    // 清理临时文件
    let _ = tokio::fs::remove_file(&temp_list_path).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {}", stderr));
    }

    Ok(())
}

// 上传文件到 Google Gemini File API
pub async fn upload_file_to_gemini(
    api_key: &str,
    file_path: &PathBuf,
) -> Result<GeminiFile, String> {
    let client = reqwest::Client::new();
    
    // 读取文件
    let mut file = File::open(file_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // 获取文件名和 MIME 类型
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4");
    
    let mime_type = "video/mp4"; // 默认使用 video/mp4
    
    // 创建 multipart form
    // Google Gemini API 期望文件数据在 "file" 字段中
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_data)
                .file_name(file_name.to_string())
                .mime_str(mime_type)
                .map_err(|e| format!("Failed to set mime type: {}", e))?,
        );
    
    log::info!("Uploading file to Google Gemini File API: {}", file_name);
    
    // 上传文件
    let response = client
        .post("https://generativelanguage.googleapis.com/upload/v1beta/files")
        .query(&[("key", api_key)])
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload file: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini File API error: {} - {}", status, error_text));
    }
    
    let upload_response: GeminiFileUploadResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;
    
    log::info!("File uploaded successfully: {}", upload_response.file.name);
    log::info!("File URI: {}, State: {}", upload_response.file.uri, upload_response.file.state);
    
    Ok(upload_response.file)
}

// 等待文件处理完成（ACTIVE 状态）
pub async fn wait_until_active(
    api_key: &str,
    file_name: &str,
    interval_ms: u64,
    timeout_ms: u64,
) -> Result<GeminiFile, String> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    
    log::info!("Waiting for file to become ACTIVE: {}", file_name);
    
    loop {
        // 获取文件状态
        // file_name 格式可能是 "files/xxx" 或只是 "xxx"，需要统一处理
        let file_id = if file_name.starts_with("files/") {
            file_name.to_string()
        } else {
            format!("files/{}", file_name)
        };
        let url = format!("https://generativelanguage.googleapis.com/v1beta/{}", file_id);
        log::debug!("Checking file status: {} (file_id: {})", url, file_id);
        
        let response = client
            .get(&url)
            .query(&[("key", api_key)])
            .send()
            .await
            .map_err(|e| format!("Failed to get file status: {}", e))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Failed to get file status: {} - {}", status, error_text);
            return Err(format!("Gemini File API error: {} - {}", status, error_text));
        }
        
        // Try to parse as direct File object first, then as wrapped response
        let response_text = response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        if response_text.is_empty() {
            return Err(format!("Empty response body from Gemini File API for file: {}", file_id));
        }
        
        log::info!("File status response (first 500 chars): {}", 
            if response_text.len() > 500 { 
                &response_text[..500] 
            } else { 
                &response_text 
            });
        
        // Try parsing as direct File object (GET endpoint returns File directly)
        let file: GeminiFile = match serde_json::from_str::<GeminiFile>(&response_text) {
            Ok(f) => f,
            Err(e1) => {
                log::debug!("Failed to parse as direct File object: {}", e1);
                // Fallback: try parsing as wrapped response { file: File }
                let file_response: GeminiFileGetResponse = serde_json::from_str(&response_text)
                    .map_err(|e2| {
                        log::error!("Failed to parse as wrapped response: {}", e2);
                        format!("Failed to parse file response. Direct parse error: {}. Wrapped parse error: {}. Response body: {}", e1, e2, response_text)
                    })?;
                file_response.file
            }
        };
        let elapsed = start_time.elapsed().as_millis();
        
        log::info!("File state: {} (elapsed: {}ms)", file.state, elapsed);
        
        match file.state.as_str() {
            "ACTIVE" => {
                log::info!("File is now ACTIVE: {} (took {}ms)", file.name, elapsed);
                return Ok(file);
            }
            "FAILED" => {
                return Err(format!("File processing failed: {}", file.name));
            }
            "PROCESSING" | "STATE_UNSPECIFIED" | "" => {
                // 文件正在处理中，继续等待
                log::debug!("File is processing, waiting {}ms...", interval_ms);
            }
            _ => {
                log::warn!("Unknown file state: {}, continuing to wait...", file.state);
            }
        }
        
        // 检查超时
        if elapsed > timeout_ms as u128 {
            return Err(format!("Wait for file ACTIVE timeout after {}ms", timeout_ms));
        }
        
        // 等待一段时间后重试
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
}

// 使用文件 URI 生成内容
pub async fn generate_content_with_file_uri(
    api_key: &str,
    model: &str,
    file_uri: &str,
    mime_type: &str,
    prompt: &str,
    resolution: &str, // "low" or "default"
) -> Result<ApiRequestResult, String> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    
    // 构建请求体
    // 根据 Google 文档：
    // - 低分辨率 (low): 约 100 tokens/秒 (66 tokens/帧 + 32 tokens/秒音频)
    // - 默认分辨率: 约 300 tokens/秒 (258 tokens/帧 + 32 tokens/秒音频)
    // 使用 low 分辨率可以减少约 66% 的 token 消耗
    // 使用 default 分辨率可以提高文字识别精度（如价格、数字等）
    // mediaResolution 应该在 part 对象中，与 fileData 同级
    let media_resolution_level = if resolution == "default" {
        "MEDIA_RESOLUTION_DEFAULT"
    } else {
        "MEDIA_RESOLUTION_LOW"
    };
    
    let request_body = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "fileData": {
                        "fileUri": file_uri,
                        "mimeType": mime_type
                    },
                    "mediaResolution": {
                        "level": media_resolution_level
                    }
                },
                {
                    "text": prompt
                }
            ]
        }]
    });
    
    log::debug!("Request body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());
    
    log::info!("Calling Google Gemini API with file URI: {}", file_uri);
    
    let response = client
        .post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model))
        .query(&[("key", api_key)])
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let status = response.status();
    
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API error: {} - {}", status, error_text));
    }
    
    let api_response: GeminiGenerateContentResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    if let Some(candidate) = api_response.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            if let Some(text) = &part.text {
                return Ok(ApiRequestResult {
                    content: text.clone(),
                    prompt_tokens: api_response.usage_metadata.as_ref().and_then(|u| u.prompt_token_count),
                    completion_tokens: api_response.usage_metadata.as_ref().and_then(|u| u.candidates_token_count),
                    total_tokens: api_response.usage_metadata.as_ref().and_then(|u| u.total_token_count),
                    status_code: status.as_u16(),
                    duration_ms,
                });
            }
        }
    }
    
    Err("No response from Gemini API".to_string())
}

// 主要的视频摘要函数：上传文件并生成摘要
pub async fn summarize_video_with_gemini(
    api_key: &str,
    video_path: &PathBuf,
    model: &str,
    prompt: &str,
    resolution: &str, // "low" or "default"
) -> Result<ApiRequestResult, String> {
    log::info!("Starting video summary with Google Gemini API (resolution: {})", resolution);
    
    // 1. 上传文件
    let uploaded_file = upload_file_to_gemini(api_key, video_path).await?;
    
    // 2. 等待文件处理完成
    log::info!("Waiting for file to become ACTIVE: {}", uploaded_file.name);
    let active_file = wait_until_active(
        api_key,
        &uploaded_file.name,
        1000, // 每 1 秒检查一次（视频文件处理可能需要更长时间）
        120_000, // 120 秒超时（2分钟，视频文件处理可能需要更长时间）
    ).await?;
    
    log::info!("File is ACTIVE, URI: {}", active_file.uri);
    
    // 3. 使用文件 URI 生成内容
    log::info!("Generating content with file URI: {} (resolution: {})", active_file.uri, resolution);
    let result = generate_content_with_file_uri(
        api_key,
        model,
        &active_file.uri,
        &active_file.mime_type,
        prompt,
        resolution,
    ).await?;
    
    log::info!("Video summary completed successfully");
    
    Ok(result)
}

// 生成文本摘要（不需要视频文件）
pub async fn generate_text_summary_with_gemini(
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    use std::time::Instant;
    use reqwest::Client;
    
    let start_time = Instant::now();
    let client = Client::new();
    
    let request_body = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "text": prompt
                }
            ]
        }]
    });
    
    log::debug!("Text summary request body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());
    
    log::info!("Calling Google Gemini API for text summary");
    
    let response = client
        .post(&format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model))
        .query(&[("key", api_key)])
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API error: {} - {}", status, error_text));
    }
    
    let api_response: GeminiGenerateContentResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    if let Some(candidate) = api_response.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            if let Some(text) = &part.text {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                log::info!("Text summary completed in {}ms", duration_ms);
                return Ok(text.clone());
            }
        }
    }
    
    Err("No response from Gemini API".to_string())
}
