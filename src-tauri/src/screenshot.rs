use chrono::Local;
use image::{ImageBuffer, Rgb, Rgba};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use tokio::time::interval;
use xcap::Monitor;

use crate::db;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Emitter};

// 获取跨平台的应用数据目录
pub fn get_app_data_dir() -> PathBuf {
    let app_name = "clarity";

    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir()
            .map(|mut p| {
                p.push(app_name);
                p.push("recordings");
                p
            })
            .unwrap_or_else(|| {
                PathBuf::from(format!(
                    "C:\\Users\\{}\\AppData\\Local\\{}\\recordings",
                    std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string()),
                    app_name
                ))
            })
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
            .unwrap_or_else(|| {
                PathBuf::from(format!(
                    "~/Library/Application Support/{}/recordings",
                    app_name
                ))
            })
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
pub async fn ensure_dir_exists(path: &Path) -> Result<(), String> {
    if !tokio::fs::metadata(path).await.is_ok() {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    Ok(())
}

// 截图并压缩保存
pub async fn capture_and_save_screenshot(
    storage_path: &Path,
    index: u64,
    db_pool: &SqlitePool,
) -> Result<(), String> {
    // 获取主屏幕并截图（在 tokio 的 blocking thread 中执行，因为 xcap 是同步的）
    // 将获取 monitors 和截图都放在同一个 spawn_blocking 中，避免生命周期问题
    let img_buffer = tokio::task::spawn_blocking(|| {
        let monitors = Monitor::all().map_err(|e| {
            format!(
                "Failed to get monitors: {}. Make sure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording",
                e
            )
        })?;

        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }

        // 使用主屏幕（第一个显示器）
        let monitor = monitors.into_iter().next().unwrap();

        #[cfg(target_os = "macos")]
        {
            eprintln!(
                "Capturing monitor: {} ({}x{})",
                monitor.name().unwrap_or_default(),
                monitor.width().unwrap_or(0),
                monitor.height().unwrap_or(0)
            );
        }

        // 截图 - 这会捕获整个屏幕，包括所有前景应用
        // xcap 使用更现代的 macOS API，应该能捕获所有窗口
        let image = monitor.capture_image().map_err(|e| {
            format!(
                "Failed to capture screen: {}. On macOS, ensure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording",
                e
            )
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
        let rgb_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(width, height, |x, y| {
                let pixel = img_buffer.get_pixel(x, y);
                Rgb([pixel[0], pixel[1], pixel[2]])
            });

        let mut output = Vec::new();
        {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 85);
            encoder
                .encode(&rgb_buffer, width, height, image::ExtendedColorType::Rgb8)
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
pub async fn screenshot_loop(
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
