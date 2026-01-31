use crate::commands::summary::video_summary_loop;
use crate::screenshot;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotStatus {
    pub is_recording: bool,
    pub screenshots_count: u64,
    pub storage_path: String,
}

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<ScreenshotStatus, String> {
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
        screenshot::screenshot_loop(
            storage_path_screenshot,
            is_recording_clone.clone(),
            screenshots_count_clone,
            db_pool.clone(),
            app_handle_screenshot,
        )
        .await;
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
        video_summary_loop(
            storage_path_summary,
            db_pool_summary,
            is_recording_summary,
            api_key_summary,
            summary_interval_summary,
            app_handle_summary,
            ai_model_summary,
            _ai_prompt_summary,
            video_resolution_summary,
        )
        .await;
        log::warn!("Video summary loop exited unexpectedly");
    });

    // 监控总结任务（如果出错会记录日志）
    tokio::spawn(async move {
        if let Err(e) = summary_handle.await {
            log::error!("Video summary task panicked: {:?}", e);
        }
    });

    *state.handle.lock().await = Some(handle);

    let storage_path_str = state
        .storage_path
        .lock()
        .await
        .to_string_lossy()
        .to_string();

    Ok(ScreenshotStatus {
        is_recording: true,
        screenshots_count: 0,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<ScreenshotStatus, String> {
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
    let storage_path_str = state
        .storage_path
        .lock()
        .await
        .to_string_lossy()
        .to_string();

    Ok(ScreenshotStatus {
        is_recording: false,
        screenshots_count,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<ScreenshotStatus, String> {
    let is_recording = *state.is_recording.lock().await;
    let screenshots_count = *state.screenshots_count.lock().await;
    let storage_path_str = state
        .storage_path
        .lock()
        .await
        .to_string_lossy()
        .to_string();

    Ok(ScreenshotStatus {
        is_recording,
        screenshots_count,
        storage_path: storage_path_str,
    })
}

#[tauri::command]
pub async fn get_storage_path(state: State<'_, AppState>) -> Result<String, String> {
    let storage_path_str = state
        .storage_path
        .lock()
        .await
        .to_string_lossy()
        .to_string();
    Ok(storage_path_str)
}

#[tauri::command]
pub async fn test_screenshot() -> Result<String, String> {
    // 测试截图功能，返回截图信息
    let result = tokio::task::spawn_blocking(|| {
        let monitors = xcap::Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;

        if monitors.is_empty() {
            return Err("No monitors found".to_string());
        }

        let monitor = monitors.into_iter().next().unwrap();
        let display_info = format!(
            "Monitor: {}, Size: {}x{}, Scale: {}",
            monitor.name().unwrap_or_default(),
            monitor.width().unwrap_or(0),
            monitor.height().unwrap_or(0),
            monitor.scale_factor().unwrap_or(1.0)
        );

        // 尝试截图
        let image = monitor.capture_image().map_err(|e| {
            format!(
                "Capture failed: {}. On macOS, ensure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording",
                e
            )
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

        Ok(format!(
            "{} | Captured: {}x{} pixels | Non-zero: {:.1}% | Unique colors: {}{}",
            display_info, width, height, non_zero_percent, unique_colors.len(), permission_hint
        ))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    Ok(result)
}
