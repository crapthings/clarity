use crate::db;
use crate::state::AppState;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayStatistics {
    pub screenshot_count: i64,
    pub summary_count: i64,
    pub api_statistics: db::ApiStatistics,
}

// 查询截图记录
#[tauri::command]
pub async fn get_traces(
    state: State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<db::ScreenshotTrace>, String> {
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
pub async fn get_summaries(
    state: State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<db::Summary>, String> {
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
pub async fn add_summary(
    state: State<'_, AppState>,
    start_time: String,
    end_time: String,
    content: String,
    screenshot_count: i32,
) -> Result<i64, String> {
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
pub async fn get_today_count(state: State<'_, AppState>) -> Result<i64, String> {
    db::get_today_screenshot_count(&state.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取 API 统计信息
#[tauri::command]
pub async fn get_api_statistics(
    state: State<'_, AppState>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<db::ApiStatistics, String> {
    let start_dt = if let Some(st) = start_time {
        Some(
            DateTime::parse_from_rfc3339(&st)
                .map_err(|e| format!("Invalid start_time format: {}", e))?
                .with_timezone(&Local),
        )
    } else {
        None
    };

    let end_dt = if let Some(et) = end_time {
        Some(
            DateTime::parse_from_rfc3339(&et)
                .map_err(|e| format!("Invalid end_time format: {}", e))?
                .with_timezone(&Local),
        )
    } else {
        None
    };

    db::get_api_statistics(&state.db_pool, start_dt, end_dt)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// 获取今天的统计概览
#[tauri::command]
pub async fn get_today_statistics(state: State<'_, AppState>) -> Result<TodayStatistics, String> {
    let today_start = Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_dt = today_start.and_local_timezone(Local).unwrap();
    let today_end_dt = Local::now();

    log::info!(
        "Getting today statistics from {} to {}",
        today_start_dt.to_rfc3339(),
        today_end_dt.to_rfc3339()
    );

    // 获取截图数量
    let screenshot_count = db::get_today_screenshot_count(&state.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 获取总结数量
    let summaries = db::get_summaries(
        &state.db_pool,
        Some(today_start_dt),
        Some(today_end_dt),
        None,
    )
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    // 获取 API 统计
    let api_stats =
        db::get_api_statistics(&state.db_pool, Some(today_start_dt), Some(today_end_dt))
            .await
            .map_err(|e| format!("Database error: {}", e))?;

    log::info!(
        "API statistics: total_requests={}, successful={}, failed={}, tokens={}",
        api_stats.total_requests,
        api_stats.successful_requests,
        api_stats.failed_requests,
        api_stats.total_tokens
    );

    Ok(TodayStatistics {
        screenshot_count,
        summary_count: summaries.len() as i64,
        api_statistics: api_stats,
    })
}

// 读取截图文件并返回 base64
#[tauri::command]
pub async fn read_screenshot_file(file_path: String) -> Result<String, String> {
    use tokio::fs;

    let path = PathBuf::from(&file_path);
    
    // 检查文件是否存在
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // 读取文件内容
    let file_data = fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // 转换为 base64
    let base64 = general_purpose::STANDARD.encode(&file_data);
    Ok(format!("data:image/jpeg;base64,{}", base64))
}
