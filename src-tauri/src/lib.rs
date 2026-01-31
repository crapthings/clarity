mod db;
mod video_summary;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use chrono::{DateTime, Local};
use image::{ImageBuffer, Rgb, Rgba};
use xcap::Monitor;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotStatus {
    pub is_recording: bool,
    pub screenshots_count: u64,
    pub storage_path: String,
}

// 全局状态管理
struct AppState {
    is_recording: Arc<Mutex<bool>>,
    screenshots_count: Arc<Mutex<u64>>,
    storage_path: Arc<Mutex<PathBuf>>,
    handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    db_pool: SqlitePool,
    gemini_api_key: Arc<Mutex<Option<String>>>,
    summary_interval_seconds: Arc<Mutex<u64>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
    ai_model: Arc<Mutex<String>>,
    _ai_prompt: Arc<Mutex<String>>,
    language: Arc<Mutex<String>>,
    video_resolution: Arc<Mutex<String>>, // "low" or "default"
}

impl AppState {
    async fn new() -> Result<Self, sqlx::Error> {
        let db_pool = db::init_db().await?;
        
        // 从数据库加载 API key
        let api_key = load_api_key_from_db(&db_pool).await.ok();
        
        // 从数据库加载总结间隔（默认 45 秒）
        let summary_interval = load_summary_interval_from_db(&db_pool).await.unwrap_or(45);
        
        // 从数据库加载 AI 模型（默认 gemini-3-flash-preview）
        let ai_model = load_ai_model_from_db(&db_pool).await.unwrap_or_else(|_| "gemini-3-flash-preview".to_string());
        
        // 从数据库加载视频分辨率设置（默认 low，节省 token）
        let video_resolution = load_video_resolution_from_db(&db_pool).await.unwrap_or_else(|_| "low".to_string());
        
        // 从数据库加载 AI 提示词（默认根据系统语言，如果没有则使用中文）
        // 优化后的 prompt：更聚焦于效率分析，减少不必要的描述
        let default_prompt_zh = "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string();
        let _default_prompt_en = "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string();
        
        // 尝试加载中文提示词，如果没有则使用默认值
        let ai_prompt = load_ai_prompt_from_db(&db_pool, Some("zh")).await
            .unwrap_or_else(|_| default_prompt_zh.clone());
        
        // 从数据库加载语言设置（默认中文）
        let language = load_language_from_db(&db_pool).await.unwrap_or_else(|_| "zh".to_string());
        
        Ok(Self {
            is_recording: Arc::new(Mutex::new(false)),
            screenshots_count: Arc::new(Mutex::new(0)),
            storage_path: Arc::new(Mutex::new(get_app_data_dir())),
            handle: Arc::new(Mutex::new(None)),
            db_pool: db_pool.clone(),
            gemini_api_key: Arc::new(Mutex::new(api_key)),
            summary_interval_seconds: Arc::new(Mutex::new(summary_interval)),
            app_handle: Arc::new(Mutex::new(None)),
            ai_model: Arc::new(Mutex::new(ai_model)),
            _ai_prompt: Arc::new(Mutex::new(ai_prompt)),
            language: Arc::new(Mutex::new(language)),
            video_resolution: Arc::new(Mutex::new(video_resolution)),
        })
    }
    
    // 发送统计更新事件
    async fn emit_statistics_updated(&self) {
        if let Some(handle) = self.app_handle.lock().await.as_ref() {
            let _ = handle.emit("statistics-updated", ());
        }
    }
}

// 从数据库加载 API key
async fn load_api_key_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'gemini_api_key' LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存 API key 到数据库
async fn save_api_key_to_db(pool: &SqlitePool, api_key: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    // 插入或更新
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('gemini_api_key', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(api_key)
    .execute(pool)
    .await?;
    
    Ok(())
}

// 从数据库加载视频分辨率设置
async fn load_video_resolution_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'video_resolution' LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存视频分辨率设置到数据库
async fn save_video_resolution_to_db(pool: &SqlitePool, resolution: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('video_resolution', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(resolution)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载 AI 模型
async fn load_ai_model_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_model' LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存 AI 模型到数据库
async fn save_ai_model_to_db(pool: &SqlitePool, model: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('ai_model', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(model)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载语言设置
async fn load_language_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'language' LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some((lang,)) = result {
        // 验证语言值是否有效
        if lang == "en" || lang == "zh" {
            Ok(lang)
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

// 保存语言设置到数据库
async fn save_language_to_db(pool: &SqlitePool, language: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('language', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(language)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载 AI 提示词

// 保存 AI 提示词到数据库（按语言）
async fn save_ai_prompt_to_db(pool: &SqlitePool, prompt: &str, language: Option<&str>) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    let key = match language {
        Some("zh") => "ai_prompt_zh",
        Some("en") => "ai_prompt_en",
        _ => "ai_prompt" // 默认兼容旧版本
    };
    
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(key)
    .bind(prompt)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载 AI 提示词（按语言）
async fn load_ai_prompt_from_db(pool: &SqlitePool, language: Option<&str>) -> Result<String, sqlx::Error> {
    let key = match language {
        Some("zh") => "ai_prompt_zh",
        Some("en") => "ai_prompt_en",
        _ => "ai_prompt" // 默认兼容旧版本
    };
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = ?1 LIMIT 1"
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    
    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 从数据库加载总结间隔
async fn load_summary_interval_from_db(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'summary_interval_seconds' LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some((value,)) = result {
        value.parse::<u64>()
            .map_err(|_| sqlx::Error::Decode("Invalid summary interval format".into()))
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

// 保存总结间隔到数据库
async fn save_summary_interval_to_db(pool: &SqlitePool, interval_seconds: u64) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;
    
    // 插入或更新
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('summary_interval_seconds', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(interval_seconds.to_string())
    .execute(pool)
    .await?;
    
    Ok(())
}

// 获取跨平台的应用数据目录
fn get_app_data_dir() -> PathBuf {
    let app_name = "clarity";
    
    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir()
            .map(|mut p| {
                p.push(app_name);
                p.push("recordings");
                p
            })
            .unwrap_or_else(|| PathBuf::from(format!("C:\\Users\\{}\\AppData\\Local\\{}\\recordings", 
                std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string()), app_name)))
    }
    
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|mut p| {
                p.push("Library");
                p.push("Application Support");
                p.push(app_name);
                p.push("recordings");
                p
            })
            .unwrap_or_else(|| PathBuf::from(format!("~/Library/Application Support/{}/recordings", app_name)))
    }
    
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .map(|mut p| {
                p.push(".local");
                p.push("share");
                p.push(app_name);
                p.push("recordings");
                p
            })
            .unwrap_or_else(|| PathBuf::from(format!("~/.local/share/{}/recordings", app_name)))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from(format!("./{}", app_name))
    }
}

// 确保目录存在
async fn ensure_dir_exists(path: &Path) -> Result<(), String> {
    if !tokio::fs::metadata(path).await.is_ok() {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    Ok(())
}

// 截图并压缩保存
async fn capture_and_save_screenshot(
    storage_path: &Path,
    index: u64,
    db_pool: &SqlitePool,
) -> Result<(), String> {
    // 获取主屏幕并截图（在 tokio 的 blocking thread 中执行，因为 xcap 是同步的）
    // 将获取 monitors 和截图都放在同一个 spawn_blocking 中，避免生命周期问题
    let img_buffer = tokio::task::spawn_blocking(|| {
        let monitors = Monitor::all().map_err(|e| {
            format!("Failed to get monitors: {}. Make sure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording", e)
        })?;
        
        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }
        
        // 使用主屏幕（第一个显示器）
        let monitor = monitors.into_iter().next().unwrap();
        
        #[cfg(target_os = "macos")]
        {
            eprintln!("Capturing monitor: {} ({}x{})", 
                monitor.name().unwrap_or_default(), 
                monitor.width().unwrap_or(0), 
                monitor.height().unwrap_or(0));
        }
        
        // 截图 - 这会捕获整个屏幕，包括所有前景应用
        // xcap 使用更现代的 macOS API，应该能捕获所有窗口
        let image = monitor.capture_image().map_err(|e| {
            format!("Failed to capture screen: {}. On macOS, ensure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording", e)
        })?;
        
        #[cfg(target_os = "macos")]
        {
            eprintln!("Captured image: {}x{} pixels", image.width(), image.height());
        }
        
        // xcap 直接返回 RgbaImage (ImageBuffer<Rgba<u8>, Vec<u8>>)
        Ok::<ImageBuffer<Rgba<u8>, Vec<u8>>, String>(image)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;
    
    let width = img_buffer.width();
    let height = img_buffer.height();
    
    // 生成文件名（使用时间戳和索引）
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H-%M-%S").to_string();
    let filename = format!("{}_{}_{:06}.jpg", date_str, time_str, index);
    
    // 创建日期目录
    let date_dir = storage_path.join(&date_str);
    ensure_dir_exists(&date_dir).await?;
    
    let file_path = date_dir.join(&filename);
    
    // 压缩并保存（JPEG 质量 85，平衡质量和文件大小）
    // JPEG 不支持 RGBA，需要转换为 RGB
    // 在 blocking thread 中执行图片编码
    let output = tokio::task::spawn_blocking(move || {
        // 将 RGBA 转换为 RGB（去掉 alpha 通道）
        let rgb_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(
            width,
            height,
            |x, y| {
                let pixel = img_buffer.get_pixel(x, y);
                Rgb([pixel[0], pixel[1], pixel[2]])
            },
        );
        
        let mut output = Vec::new();
        {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 85);
            encoder
                .encode(
                    &rgb_buffer,
                    width,
                    height,
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("Failed to encode image: {}", e))?;
        }
        Ok::<Vec<u8>, String>(output)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;
    
    tokio::fs::write(&file_path, output)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;
    
    // 获取文件大小
    let file_size = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len() as i64;
    
    // 保存到数据库
    let timestamp = Local::now();
    let file_path_str = file_path.to_string_lossy().to_string();
    
    if let Err(e) = db::insert_screenshot_trace(
        db_pool,
        timestamp,
        file_path_str,
        width as i32,
        height as i32,
        file_size,
    )
    .await
    {
        eprintln!("Failed to insert screenshot trace to database: {}", e);
        // 不返回错误，因为文件已经保存成功
    }
    
    Ok(())
}

// 截图循环任务
async fn screenshot_loop(
    storage_path: PathBuf,
    is_recording: Arc<Mutex<bool>>,
    screenshots_count: Arc<Mutex<u64>>,
    db_pool: SqlitePool,
    app_handle: Option<AppHandle>,
) {
    let mut interval = interval(StdDuration::from_secs(1)); // 1秒 = 1fps
    let mut index = 0u64;
    
    // 确保目录存在
    if let Err(e) = ensure_dir_exists(&storage_path).await {
        eprintln!("Failed to create storage directory: {}", e);
        return;
    }
    
    loop {
        interval.tick().await;
        
        // 检查是否还在录制
        let recording = *is_recording.lock().await;
        if !recording {
            break;
        }
        
        // 执行截图
        match capture_and_save_screenshot(&storage_path, index, &db_pool).await {
            Ok(_) => {
                index += 1;
                *screenshots_count.lock().await = index;
                // 发送统计更新事件
                if let Some(handle) = app_handle.as_ref() {
                    let _ = handle.emit("statistics-updated", ());
                }
            }
            Err(e) => {
                eprintln!("Screenshot error: {}", e);
            }
        }
    }
}

// 视频总结任务
async fn video_summary_loop(
    storage_path: PathBuf,
    db_pool: SqlitePool,
    is_recording: Arc<Mutex<bool>>,
    gemini_api_key: Arc<Mutex<Option<String>>>,
    summary_interval_seconds: Arc<Mutex<u64>>,
    app_handle: Option<AppHandle>,
    ai_model: Arc<Mutex<String>>,
    _ai_prompt: Arc<Mutex<String>>,
    video_resolution: Arc<Mutex<String>>,
) {
    log::info!("Video summary loop started");
    let mut current_interval = *summary_interval_seconds.lock().await;
    let mut interval_timer = interval(StdDuration::from_secs(current_interval));
    // 跳过第一次立即触发，等待完整的间隔时间
    interval_timer.tick().await;
    log::info!("Video summary interval set to {} seconds", current_interval);
    
    loop {
        interval_timer.tick().await;
        log::debug!("Video summary tick");
        
        // 检查是否还在录制
        let recording = *is_recording.lock().await;
        if !recording {
            log::debug!("Recording is not active, skipping video summary");
            continue;
        }
        
        // 检查间隔是否已更改，如果是则重新创建定时器
        let new_interval = *summary_interval_seconds.lock().await;
        if new_interval != current_interval {
            log::info!("Summary interval changed from {} to {} seconds", current_interval, new_interval);
            current_interval = new_interval;
            interval_timer = interval(StdDuration::from_secs(current_interval));
            continue; // 跳过本次，等待新的间隔
        }
        
        // 检查 API key
        let api_key = gemini_api_key.lock().await.clone();
        if api_key.is_none() {
            log::warn!("Google Gemini API key not set, skipping video summary");
            continue;
        }
        let api_key = api_key.unwrap();
        log::info!("Starting video summary for last {} seconds", current_interval);
        
        // 获取最近 N 秒的截图（N = summary_interval_seconds）
        let seconds_ago = Local::now() - chrono::Duration::seconds(current_interval as i64);
        match db::get_screenshot_traces(&db_pool, Some(seconds_ago), None, None).await {
            Ok(traces) => {
                if traces.is_empty() {
                    log::warn!("No screenshots in the last {} seconds", current_interval);
                    continue;
                }
                
                log::info!("Found {} screenshots to process", traces.len());
                
                // 创建视频
                let video_path = storage_path
                    .join("videos")
                    .join(format!("summary_{}.mp4", Local::now().format("%Y%m%d_%H%M%S")));
                
                // 确保视频目录存在
                if let Some(parent) = video_path.parent() {
                    if let Err(e) = ensure_dir_exists(parent).await {
                        log::error!("Failed to create video directory: {}", e);
                        continue;
                    }
                }
                
                let image_paths: Vec<PathBuf> = traces.iter()
                    .map(|t| PathBuf::from(&t.file_path))
                    .collect();
                
                log::info!("Creating video from {} images", image_paths.len());
                match video_summary::create_video_from_images(&image_paths, &video_path, 1).await {
                    Ok(_) => {
                        log::info!("Video created successfully: {}", video_path.display());
                        
                        // 调用 Google Gemini API（使用 File API）
                        log::info!("Calling Google Gemini API for video summary");
                        let model = ai_model.lock().await.clone();
                        
                        // 根据当前语言从数据库加载提示词
                        let current_language = {
                            // 尝试从数据库加载语言设置，如果没有则默认中文
                            let lang_result = load_language_from_db(&db_pool).await.unwrap_or_else(|_| "zh".to_string());
                            lang_result
                        };
                        
                        // 从数据库加载当前语言的提示词
                        let prompt = load_ai_prompt_from_db(&db_pool, Some(&current_language)).await
                            .unwrap_or_else(|_| {
                                if current_language == "en" {
                                    "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string()
                                } else {
                                    "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string()
                                }
                            });
                        
                        // 获取视频分辨率设置
                        let resolution = video_resolution.lock().await.clone();
                        
                        match video_summary::summarize_video_with_gemini(
                            &api_key,
                            &video_path,
                            &model,
                            &prompt,
                            &resolution,
                        ).await {
                            Ok(result) => {
                                log::info!("Summary generated successfully, length: {} chars", result.content.len());
                                log::info!("Token usage: prompt={:?}, completion={:?}, total={:?}", 
                                    result.prompt_tokens, result.completion_tokens, result.total_tokens);
                                
                                // 记录 API 请求到数据库
                                if let Err(e) = db::insert_api_request(
                                    &db_pool,
                                    &model,
                                    "https://generativelanguage.googleapis.com/v1beta/models",
                                    result.prompt_tokens,
                                    result.completion_tokens,
                                    result.total_tokens,
                                    result.status_code,
                                    true,
                                    None,
                                    result.duration_ms,
                                ).await {
                                    log::error!("Failed to save API request to database: {}", e);
                                } else {
                                    // API 请求保存成功，发送统计更新事件
                                    if let Some(handle) = app_handle.as_ref() {
                                        let _ = handle.emit("statistics-updated", ());
                                    }
                                }
                                
                                // 保存摘要到数据库
                                // 确保时间顺序正确：start_time 应该是最早的，end_time 应该是最晚的
                                // traces 是按 timestamp DESC 排序的，所以需要找到最小和最大时间
                                let mut timestamps: Vec<DateTime<Local>> = traces.iter().map(|t| t.timestamp).collect();
                                timestamps.sort(); // 按时间升序排序
                                let start_time = timestamps.first().unwrap().clone(); // 最早的时间
                                let end_time = timestamps.last().unwrap().clone(); // 最晚的时间
                                let screenshot_count = traces.len() as i32;
                                
                                match db::insert_summary(
                                    &db_pool,
                                    start_time,
                                    end_time,
                                    result.content,
                                    screenshot_count,
                                ).await {
                                    Ok(id) => {
                                        log::info!("Summary saved to database with id: {}", id);
                                        // 总结保存成功，发送统计更新事件
                                        if let Some(handle) = app_handle.as_ref() {
                                            let _ = handle.emit("statistics-updated", ());
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to save summary to database: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to summarize video with Google Gemini: {}", e);
                                
                                // 记录失败的 API 请求
                                let error_msg = e.clone();
                                if db::insert_api_request(
                                    &db_pool,
                                    &model,
                                    "https://generativelanguage.googleapis.com/v1beta/models",
                                    None,
                                    None,
                                    None,
                                    0,
                                    false,
                                    Some(&error_msg),
                                    0,
                                ).await.is_ok() {
                                    // API 请求记录保存成功，发送统计更新事件
                                    if let Some(handle) = app_handle.as_ref() {
                                        let _ = handle.emit("statistics-updated", ());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create video from images: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get screenshot traces from database: {}", e);
            }
        }
    }
}

#[tauri::command]
async fn start_recording(state: tauri::State<'_, AppState>) -> Result<ScreenshotStatus, String> {
    log::info!("Starting recording");
    let mut is_recording = state.is_recording.lock().await;
    
    if *is_recording {
        log::warn!("Recording is already in progress");
        return Err("Recording is already in progress".to_string());
    }
    
    *is_recording = true;
    log::info!("Recording started");
    
    let storage_path = state.storage_path.lock().await.clone();
    let is_recording_clone = state.is_recording.clone();
    let screenshots_count_clone = state.screenshots_count.clone();
    let db_pool = state.db_pool.clone();
    
    // 克隆 storage_path 用于两个任务
    let storage_path_screenshot = storage_path.clone();
    let storage_path_summary = storage_path.clone();
    
    // 启动截图任务
    let app_handle_screenshot = state.app_handle.lock().await.clone();
    let handle = tokio::spawn(async move {
        screenshot_loop(storage_path_screenshot, is_recording_clone.clone(), screenshots_count_clone, db_pool.clone(), app_handle_screenshot).await;
    });
    
    // 启动视频总结任务
    let db_pool_summary = state.db_pool.clone();
    let is_recording_summary = state.is_recording.clone();
    let api_key_summary = state.gemini_api_key.clone();
    let summary_interval_summary = state.summary_interval_seconds.clone();
    let app_handle_summary = state.app_handle.lock().await.clone();
    let ai_model_summary = state.ai_model.clone();
    // 注意：ai_prompt 不再需要传递，因为 video_summary_loop 会根据语言从数据库加载
    let _ai_prompt_summary = state._ai_prompt.clone(); // 保留以兼容函数签名，但实际不再使用
    let video_resolution_summary = state.video_resolution.clone();
    let summary_handle = tokio::spawn(async move {
        log::info!("Starting video summary background task");
        video_summary_loop(storage_path_summary, db_pool_summary, is_recording_summary, api_key_summary, summary_interval_summary, app_handle_summary, ai_model_summary, _ai_prompt_summary, video_resolution_summary).await;
        log::warn!("Video summary loop exited unexpectedly");
    });
    
    // 监控总结任务（如果出错会记录日志）
    tokio::spawn(async move {
        if let Err(e) = summary_handle.await {
            log::error!("Video summary task panicked: {:?}", e);
        }
    });
    
    *state.handle.lock().await = Some(handle);
    
    let storage_path_str = state.storage_path.lock().await.to_string_lossy().to_string();
    
    Ok(ScreenshotStatus {
        is_recording: true,
        screenshots_count: 0,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<ScreenshotStatus, String> {
    let mut is_recording = state.is_recording.lock().await;
    
    if !*is_recording {
        return Err("Recording is not in progress".to_string());
    }
    
    *is_recording = false;
    
    // 等待任务完成
    if let Some(handle) = state.handle.lock().await.take() {
        handle.abort();
    }
    
    let screenshots_count = *state.screenshots_count.lock().await;
    let storage_path_str = state.storage_path.lock().await.to_string_lossy().to_string();
    
    Ok(ScreenshotStatus {
        is_recording: false,
        screenshots_count,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<ScreenshotStatus, String> {
    let is_recording = *state.is_recording.lock().await;
    let screenshots_count = *state.screenshots_count.lock().await;
    let storage_path_str = state.storage_path.lock().await.to_string_lossy().to_string();
    
    Ok(ScreenshotStatus {
        is_recording,
        screenshots_count,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
async fn get_storage_path(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let storage_path_str = state.storage_path.lock().await.to_string_lossy().to_string();
    Ok(storage_path_str)
}

#[tauri::command]
async fn test_screenshot() -> Result<String, String> {
    // 测试截图功能，返回截图信息
    let result = tokio::task::spawn_blocking(|| {
        let monitors = Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;
        
        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }
        
        let monitor = monitors.into_iter().next().unwrap();
        let display_info = format!("Monitor: {}, Size: {}x{}, Scale: {}", 
            monitor.name().unwrap_or_default(),
            monitor.width().unwrap_or(0), 
            monitor.height().unwrap_or(0),
            monitor.scale_factor().unwrap_or(1.0));
        
        // 尝试截图
        let image = monitor.capture_image().map_err(|e| {
            format!("Capture failed: {}. On macOS, ensure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording", e)
        })?;
        
        let width = image.width();
        let height = image.height();
        
        // 检查图片是否全黑或全透明（通常表示权限问题）
        let pixels = image.as_raw();
        let total_pixels = (width * height) as usize;
        let mut non_zero_count = 0;
        let mut unique_colors = std::collections::HashSet::new();
        
        for chunk in pixels.chunks(4) {
            if chunk.len() == 4 {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];
                if r != 0 || g != 0 || b != 0 {
                    non_zero_count += 1;
                }
                // 采样一些颜色
                if unique_colors.len() < 100 {
                    unique_colors.insert((r, g, b));
                }
            }
        }
        
        let non_zero_percent = (non_zero_count as f64 / total_pixels as f64) * 100.0;
        
        let permission_hint = if non_zero_percent < 1.0 || unique_colors.len() < 5 {
            " ⚠️ WARNING: Image appears mostly blank! This usually means Screen Recording permission is NOT properly granted. In dev mode, grant permission to Terminal/Cursor/iTerm, not just 'clarity'."
        } else {
            " ✅ Image has content"
        };
        
        Ok(format!("{} | Captured: {}x{} pixels | Non-zero: {:.1}% | Unique colors: {}{}", 
            display_info, width, height, non_zero_percent, unique_colors.len(), permission_hint))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;
    
    Ok(result)
}

// 查询截图记录
#[tauri::command]
async fn get_traces(
    state: tauri::State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<db::ScreenshotTrace>, String> {
    use chrono::DateTime;
    
    let start_dt = start_time
        .map(|s| DateTime::parse_from_rfc3339(&s))
        .transpose()
        .map_err(|e| format!("Invalid start_time format: {}", e))?
        .map(|dt| dt.with_timezone(&Local));
    
    let end_dt = end_time
        .map(|s| DateTime::parse_from_rfc3339(&s))
        .transpose()
        .map_err(|e| format!("Invalid end_time format: {}", e))?
        .map(|dt| dt.with_timezone(&Local));
    
    db::get_screenshot_traces(&state.db_pool, start_dt, end_dt, limit)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 查询摘要
#[tauri::command]
async fn get_summaries(
    state: tauri::State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<db::Summary>, String> {
    use chrono::DateTime;
    
    let start_dt = start_time
        .map(|s| DateTime::parse_from_rfc3339(&s))
        .transpose()
        .map_err(|e| format!("Invalid start_time format: {}", e))?
        .map(|dt| dt.with_timezone(&Local));
    
    let end_dt = end_time
        .map(|s| DateTime::parse_from_rfc3339(&s))
        .transpose()
        .map_err(|e| format!("Invalid end_time format: {}", e))?
        .map(|dt| dt.with_timezone(&Local));
    
    db::get_summaries(&state.db_pool, start_dt, end_dt, limit)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 添加摘要
#[tauri::command]
async fn add_summary(
    state: tauri::State<'_, AppState>,
    start_time: String,
    end_time: String,
    content: String,
    screenshot_count: i32,
) -> Result<i64, String> {
    use chrono::DateTime;
    
    let start_dt = DateTime::parse_from_rfc3339(&start_time)
        .map_err(|e| format!("Invalid start_time format: {}", e))?
        .with_timezone(&Local);
    
    let end_dt = DateTime::parse_from_rfc3339(&end_time)
        .map_err(|e| format!("Invalid end_time format: {}", e))?
        .with_timezone(&Local);
    
    db::insert_summary(&state.db_pool, start_dt, end_dt, content, screenshot_count)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取今天的截图数量
#[tauri::command]
async fn get_today_count(state: tauri::State<'_, AppState>) -> Result<i64, String> {
    db::get_today_screenshot_count(&state.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取 Google Gemini API Key
#[tauri::command]
async fn get_gemini_api_key(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let api_key = state.gemini_api_key.lock().await.clone();
    Ok(api_key.unwrap_or_default())
}

// 设置 Google Gemini API Key
#[tauri::command]
async fn set_gemini_api_key(
    state: tauri::State<'_, AppState>,
    api_key: String,
) -> Result<(), String> {
    // 保存到数据库
    save_api_key_to_db(&state.db_pool, &api_key)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 更新内存中的值
    *state.gemini_api_key.lock().await = Some(api_key);
    
    Ok(())
}

// 获取总结间隔（秒）
#[tauri::command]
async fn get_summary_interval(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    let interval = *state.summary_interval_seconds.lock().await;
    log::info!("Getting summary interval: {} seconds", interval);
    Ok(interval)
}

// 设置总结间隔（秒）
#[tauri::command]
async fn set_summary_interval(
    state: tauri::State<'_, AppState>,
    interval_seconds: u64,
) -> Result<(), String> {
    log::info!("Setting summary interval to {} seconds", interval_seconds);
    
    if interval_seconds < 10 {
        return Err("Summary interval must be at least 10 seconds".to_string());
    }
    
    if interval_seconds > 3600 {
        return Err("Summary interval must be at most 3600 seconds (1 hour)".to_string());
    }
    
    // 保存到数据库
    save_summary_interval_to_db(&state.db_pool, interval_seconds)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 更新内存中的值
    *state.summary_interval_seconds.lock().await = interval_seconds;
    log::info!("Summary interval updated successfully");
    
    Ok(())
}

// 测试视频总结功能（诊断用）
#[tauri::command]
async fn test_video_summary(state: tauri::State<'_, AppState>) -> Result<String, String> {
    log::info!("Testing video summary functionality");
    
    let mut diagnostics = Vec::new();
    
    // 检查 API key
    let api_key = state.gemini_api_key.lock().await.clone();
    if api_key.is_none() {
        diagnostics.push("❌ Google Gemini API key not set".to_string());
    } else {
        diagnostics.push("✅ Google Gemini API key is set".to_string());
    }
    
    // 检查 ffmpeg
    let ffmpeg_paths = if cfg!(target_os = "macos") {
        vec!["ffmpeg", "/usr/local/bin/ffmpeg", "/opt/homebrew/bin/ffmpeg"]
    } else {
        vec!["ffmpeg"]
    };
    
    let mut ffmpeg_found = false;
    let mut ffmpeg_path = String::new();
    for path in &ffmpeg_paths {
        let check = tokio::process::Command::new(path)
            .arg("-version")
            .output()
            .await;
        
        if check.is_ok() {
            ffmpeg_found = true;
            ffmpeg_path = path.to_string();
            break;
        }
    }
    
    if ffmpeg_found {
        diagnostics.push(format!("✅ ffmpeg found at: {}", ffmpeg_path));
    } else {
        diagnostics.push(format!("❌ ffmpeg not found. Tried: {:?}", ffmpeg_paths));
    }
    
    // 检查截图数量
    let count = db::get_today_screenshot_count(&state.db_pool).await
        .map_err(|e| format!("Database error: {}", e))?;
    diagnostics.push(format!("📸 Today's screenshots: {}", count));
    
    // 检查总结间隔
    let interval = *state.summary_interval_seconds.lock().await;
    diagnostics.push(format!("⏱️ Summary interval: {} seconds", interval));
    
    // 检查是否在录制
    let recording = *state.is_recording.lock().await;
    diagnostics.push(format!("🎬 Recording: {}", if recording { "Yes" } else { "No" }));
    
    // 检查存储路径
    let storage_path = state.storage_path.lock().await.clone();
    diagnostics.push(format!("📁 Storage path: {}", storage_path.display()));
    
    let result = diagnostics.join("\n");
    log::info!("Video summary diagnostics:\n{}", result);
    Ok(result)
}

// 获取 API 统计信息
#[tauri::command]
async fn get_api_statistics(
    state: tauri::State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<db::ApiStatistics, String> {
    
    let start_dt = if let Some(st) = start_time {
        Some(DateTime::parse_from_rfc3339(&st)
            .map_err(|e| format!("Invalid start_time format: {}", e))?
            .with_timezone(&Local))
    } else {
        None
    };
    
    let end_dt = if let Some(et) = end_time {
        Some(DateTime::parse_from_rfc3339(&et)
            .map_err(|e| format!("Invalid end_time format: {}", e))?
            .with_timezone(&Local))
    } else {
        None
    };
    
    db::get_api_statistics(&state.db_pool, start_dt, end_dt)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取今天的统计概览
#[tauri::command]
async fn get_today_statistics(state: tauri::State<'_, AppState>) -> Result<TodayStatistics, String> {
    let today_start = Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_dt = today_start.and_local_timezone(Local).unwrap();
    let today_end_dt = Local::now();
    
    log::info!("Getting today statistics from {} to {}", today_start_dt.to_rfc3339(), today_end_dt.to_rfc3339());
    
    // 获取截图数量
    let screenshot_count = db::get_today_screenshot_count(&state.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 获取总结数量
    let summaries = db::get_summaries(&state.db_pool, Some(today_start_dt), Some(today_end_dt), None)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 获取 API 统计
    let api_stats = db::get_api_statistics(&state.db_pool, Some(today_start_dt), Some(today_end_dt))
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    log::info!("API statistics: total_requests={}, successful={}, failed={}, tokens={}", 
        api_stats.total_requests, api_stats.successful_requests, api_stats.failed_requests, api_stats.total_tokens);
    
    Ok(TodayStatistics {
        screenshot_count,
        summary_count: summaries.len() as i64,
        api_statistics: api_stats,
    })
}

// 获取 AI 模型
#[tauri::command]
async fn get_ai_model(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.ai_model.lock().await.clone())
}

// 设置 AI 模型
#[tauri::command]
async fn set_ai_model(
    state: tauri::State<'_, AppState>,
    model: String,
) -> Result<(), String> {
    if model.is_empty() {
        return Err("Model cannot be empty".to_string());
    }
    
    // 保存到数据库
    save_ai_model_to_db(&state.db_pool, &model)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 更新内存中的值
    *state.ai_model.lock().await = model;
    log::info!("AI model updated successfully");
    
    Ok(())
}

// 获取 AI 提示词（按语言）
#[tauri::command]
async fn get_ai_prompt(
    state: tauri::State<'_, AppState>,
    language: Option<String>,
) -> Result<String, String> {
    let lang = language.as_deref().unwrap_or("zh");
    
    // 从数据库加载指定语言的提示词
    match load_ai_prompt_from_db(&state.db_pool, Some(lang)).await {
        Ok(prompt) => Ok(prompt),
        Err(_) => {
            // 如果没有找到，返回默认提示词
            if lang == "en" {
                Ok("Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string())
            } else {
                Ok("分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string())
            }
        }
    }
}

// 设置 AI 提示词（按语言）
#[tauri::command]
async fn set_ai_prompt(
    state: tauri::State<'_, AppState>,
    prompt: String,
    language: Option<String>,
) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }
    
    let lang = language.as_deref().unwrap_or("zh");
    
    // 保存到数据库（按语言）
    save_ai_prompt_to_db(&state.db_pool, &prompt, Some(lang))
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 如果当前语言匹配，更新内存中的值
    // 注意：这里我们不再更新内存，因为内存中只存储一个值
    // 实际使用时，会根据当前语言从数据库加载
    log::info!("AI prompt updated successfully for language: {}", lang);
    
    Ok(())
}

// 恢复默认提示词（按语言）
#[tauri::command]
async fn reset_ai_prompt(
    state: tauri::State<'_, AppState>,
    language: Option<String>,
) -> Result<String, String> {
    let lang = language.as_deref().unwrap_or("zh");
    
    let default_prompt = if lang == "en" {
        "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string()
    } else {
        "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string()
    };
    
    // 保存到数据库（按语言）
    save_ai_prompt_to_db(&state.db_pool, &default_prompt, Some(lang))
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    log::info!("AI prompt reset to default for language: {}", lang);
    
    Ok(default_prompt)
}

// 获取视频分辨率设置
#[tauri::command]
async fn get_video_resolution(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.video_resolution.lock().await.clone())
}

// 设置视频分辨率
#[tauri::command]
async fn set_video_resolution(
    state: tauri::State<'_, AppState>,
    resolution: String,
) -> Result<(), String> {
    if resolution != "low" && resolution != "default" {
        return Err("Resolution must be 'low' or 'default'".to_string());
    }
    
    // 保存到数据库
    save_video_resolution_to_db(&state.db_pool, &resolution)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 更新内存中的值
    *state.video_resolution.lock().await = resolution.clone();
    log::info!("Video resolution updated to: {}", resolution);
    
    Ok(())
}

// 获取语言设置
#[tauri::command]
async fn get_language(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.language.lock().await.clone())
}

// 设置语言
#[tauri::command]
async fn set_language(
    state: tauri::State<'_, AppState>,
    language: String,
) -> Result<(), String> {
    if language != "en" && language != "zh" {
        return Err("Language must be 'en' or 'zh'".to_string());
    }
    
    // 保存到数据库
    save_language_to_db(&state.db_pool, &language)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 更新内存中的值
    *state.language.lock().await = language.clone();
    log::info!("Language updated to: {}", language);
    
    Ok(())
}

// 生成每日总结
#[tauri::command]
async fn generate_daily_summary(
    state: tauri::State<'_, AppState>,
    date: Option<String>, // YYYY-MM-DD format, if None, use today
) -> Result<db::DailySummary, String> {
    use chrono::NaiveDate;
    
    let target_date = if let Some(d) = date {
        d
    } else {
        let today = Local::now().date_naive();
        today.format("%Y-%m-%d").to_string()
    };
    
    // 解析日期
    let date_naive = NaiveDate::parse_from_str(&target_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    
    // 计算当天的开始和结束时间
    let start_time = date_naive.and_hms_opt(0, 0, 0)
        .ok_or_else(|| "Invalid date".to_string())?
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| "Invalid timezone conversion".to_string())?;
    
    let end_time = date_naive.and_hms_opt(23, 59, 59)
        .ok_or_else(|| "Invalid date".to_string())?
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| "Invalid timezone conversion".to_string())?;
    
    // 获取当天的所有摘要
    let summaries = db::get_summaries(&state.db_pool, Some(start_time), Some(end_time), None)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    // 获取当天的截图数量
    let screenshot_count = db::get_screenshot_traces(&state.db_pool, Some(start_time), Some(end_time), None)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .len() as i32;
    
    // 计算总时长（秒）
    let total_duration_seconds = summaries.iter()
        .map(|s| (s.end_time - s.start_time).num_seconds())
        .sum::<i64>();
    
    // 获取当前语言设置
    let current_language = {
        let lang_result = load_language_from_db(&state.db_pool).await.unwrap_or_else(|_| "zh".to_string());
        lang_result
    };
    
    // 获取对应语言的提示词
    let _prompt = load_ai_prompt_from_db(&state.db_pool, Some(&current_language)).await
        .unwrap_or_else(|_| {
            if current_language == "en" {
                "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string()
            } else {
                "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string()
            }
        });
    
    // 如果有摘要，合并所有摘要内容并生成每日总结
    let content = if summaries.is_empty() {
        if current_language == "en" {
            "No activity recorded for this day.".to_string()
        } else {
            "今天没有记录任何活动。".to_string()
        }
    } else {
        // 合并所有摘要内容
        let combined_content = summaries.iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // 使用 Gemini API 生成每日总结
        let api_key = state.gemini_api_key.lock().await.clone();
        if let Some(key) = api_key {
            let model = state.ai_model.lock().await.clone();
            
            // 构建提示词，要求生成每日总结
            let daily_prompt = if current_language == "en" {
                format!("Based on the following activity summaries from today, provide a comprehensive daily summary. Include: 1) Overall productivity assessment; 2) Main activities and time distribution; 3) Key insights and recommendations for improvement.\n\nToday's summaries:\n{}", combined_content)
            } else {
                format!("基于以下今天的所有活动摘要，生成一份综合的每日总结。包括：1) 整体效率评估；2) 主要活动和时间分布；3) 关键洞察和改进建议。\n\n今天的摘要：\n{}", combined_content)
            };
            
            // 调用 Gemini API（使用文本输入，不需要视频）
            match video_summary::generate_text_summary_with_gemini(&key, &model, &daily_prompt).await {
                Ok(summary_content) => summary_content,
                Err(e) => {
                    log::warn!("Failed to generate daily summary with AI: {}. Using combined summaries.", e);
                    // 如果 AI 生成失败，使用合并的摘要内容
                    combined_content
                }
            }
        } else {
            // 如果没有 API key，使用合并的摘要内容
            combined_content
        }
    };
    
    // 保存或更新每日总结
    let _id = db::upsert_daily_summary(
        &state.db_pool,
        &target_date,
        &content,
        screenshot_count,
        summaries.len() as i32,
        total_duration_seconds,
    )
    .await
    .map_err(|e| format!("Database error: {}", e))?;
    
    // 获取保存的每日总结
    let daily_summary = db::get_daily_summary(&state.db_pool, &target_date)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Failed to retrieve saved daily summary".to_string())?;
    
    Ok(daily_summary)
}

// 获取每日总结
#[tauri::command]
async fn get_daily_summary(
    state: tauri::State<'_, AppState>,
    date: Option<String>, // YYYY-MM-DD format, if None, use today
) -> Result<Option<db::DailySummary>, String> {
    
    let target_date = if let Some(d) = date {
        d
    } else {
        let today = Local::now().date_naive();
        today.format("%Y-%m-%d").to_string()
    };
    
    db::get_daily_summary(&state.db_pool, &target_date)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取历史统计数据（用于图表）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalStats {
    pub date: String, // YYYY-MM-DD
    pub screenshot_count: i64,
    pub summary_count: i64,
    pub total_duration_seconds: i64,
}

#[tauri::command]
async fn get_historical_stats(
    state: tauri::State<'_, AppState>,
    days: i64, // 获取最近多少天的数据
) -> Result<Vec<HistoricalStats>, String> {
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(days - 1);
    
    let start_date_str = start_date.format("%Y-%m-%d").to_string();
    let end_date_str = end_date.format("%Y-%m-%d").to_string();
    
    // 获取每日总结
    let daily_summaries = db::get_daily_summaries(
        &state.db_pool,
        Some(&start_date_str),
        Some(&end_date_str),
        None,
    )
    .await
    .map_err(|e| format!("Database error: {}", e))?;
    
    // 创建一个日期到统计数据的映射
    let mut stats_map: std::collections::HashMap<String, HistoricalStats> = std::collections::HashMap::new();
    
    // 填充已有的每日总结数据
    for summary in daily_summaries {
        stats_map.insert(summary.date.clone(), HistoricalStats {
            date: summary.date.clone(),
            screenshot_count: summary.screenshot_count as i64,
            summary_count: summary.summary_count as i64,
            total_duration_seconds: summary.total_duration_seconds,
        });
    }
    
    // 填充缺失的日期（如果没有每日总结，从原始数据计算）
    let mut current_date = start_date;
    let mut result: Vec<HistoricalStats> = Vec::new();
    
    while current_date <= end_date {
        let date_str = current_date.format("%Y-%m-%d").to_string();
        
        if let Some(stats) = stats_map.get(&date_str) {
            result.push(stats.clone());
        } else {
            // 如果没有每日总结，从原始数据计算
            let day_start = current_date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| "Invalid date".to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Invalid timezone conversion".to_string())?;
            
            let day_end = current_date.and_hms_opt(23, 59, 59)
                .ok_or_else(|| "Invalid date".to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Invalid timezone conversion".to_string())?;
            
            let screenshots = db::get_screenshot_traces(&state.db_pool, Some(day_start), Some(day_end), None)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
            
            let summaries = db::get_summaries(&state.db_pool, Some(day_start), Some(day_end), None)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
            
            let total_duration = summaries.iter()
                .map(|s| (s.end_time - s.start_time).num_seconds())
                .sum::<i64>();
            
            result.push(HistoricalStats {
                date: date_str,
                screenshot_count: screenshots.len() as i64,
                summary_count: summaries.len() as i64,
                total_duration_seconds: total_duration,
            });
        }
        
        current_date = current_date + chrono::Duration::days(1);
    }
    
    // 按日期排序（从旧到新）
    result.sort_by(|a, b| a.date.cmp(&b.date));
    
    Ok(result)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodayStatistics {
    screenshot_count: i64,
    summary_count: i64,
    api_statistics: db::ApiStatistics,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    log::info!("Clarity application starting");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                log::info!("Initializing application state");
                let app_state = AppState::new().await
                    .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to initialize database: {}", e)))?;
                
                // 保存 app handle 用于发送事件
                *app_state.app_handle.lock().await = Some(app.handle().clone());
                
                log::info!("Application state initialized successfully");
                app.manage(app_state);
                Ok(())
            })
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_status,
            get_storage_path,
            test_screenshot,
            get_traces,
            get_summaries,
            add_summary,
            get_today_count,
            get_gemini_api_key,
            set_gemini_api_key,
            get_summary_interval,
            set_summary_interval,
            test_video_summary,
            get_api_statistics,
            get_today_statistics,
            get_ai_model,
            set_ai_model,
            get_ai_prompt,
            set_ai_prompt,
            reset_ai_prompt,
            get_language,
            set_language,
            generate_daily_summary,
            get_daily_summary,
            get_historical_stats,
            get_video_resolution,
            set_video_resolution,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}