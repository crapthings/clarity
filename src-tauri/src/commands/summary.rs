use crate::db;
use crate::screenshot;
use crate::settings;
use crate::state::AppState;
use crate::video_summary;
use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalStats {
    pub date: String, // YYYY-MM-DD
    pub screenshot_count: i64,
    pub summary_count: i64,
    pub total_duration_seconds: i64,
}

// 视频总结任务
pub async fn video_summary_loop(
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
            log::info!(
                "Summary interval changed from {} to {} seconds",
                current_interval,
                new_interval
            );
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
        log::info!(
            "Starting video summary for last {} seconds",
            current_interval
        );

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
                let video_path = storage_path.join("videos").join(format!(
                    "summary_{}.mp4",
                    Local::now().format("%Y%m%d_%H%M%S")
                ));

                // 确保视频目录存在
                if let Some(parent) = video_path.parent() {
                    if let Err(e) = screenshot::ensure_dir_exists(parent).await {
                        log::error!("Failed to create video directory: {}", e);
                        continue;
                    }
                }

                let image_paths: Vec<PathBuf> =
                    traces.iter().map(|t| PathBuf::from(&t.file_path)).collect();

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
                            let lang_result = settings::load_language_from_db(&db_pool)
                                .await
                                .unwrap_or_else(|_| "zh".to_string());
                            lang_result
                        };

                        // 从数据库加载当前语言的提示词
                        let prompt = settings::load_ai_prompt_from_db(&db_pool, Some(&current_language)).await
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
                        )
                        .await
                        {
                            Ok(result) => {
                                log::info!(
                                    "Summary generated successfully, length: {} chars",
                                    result.content.len()
                                );
                                log::info!(
                                    "Token usage: prompt={:?}, completion={:?}, total={:?}",
                                    result.prompt_tokens,
                                    result.completion_tokens,
                                    result.total_tokens
                                );

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
                                )
                                .await
                                {
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
                                let mut timestamps: Vec<DateTime<Local>> =
                                    traces.iter().map(|t| t.timestamp).collect();
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
                                )
                                .await
                                {
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
                                )
                                .await
                                .is_ok()
                                {
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

// 生成每日总结
#[tauri::command]
pub async fn generate_daily_summary(
    state: State<'_, AppState>,
    date: Option<String>, // YYYY-MM-DD format, if None, use today
) -> Result<db::DailySummary, String> {
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
    let start_time = date_naive
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "Invalid date".to_string())?
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| "Invalid timezone conversion".to_string())?;

    let end_time = date_naive
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| "Invalid date".to_string())?
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| "Invalid timezone conversion".to_string())?;

    // 获取当天的所有摘要
    let summaries = db::get_summaries(&state.db_pool, Some(start_time), Some(end_time), None)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 获取当天的截图数量
    let screenshot_count =
        db::get_screenshot_traces(&state.db_pool, Some(start_time), Some(end_time), None)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .len() as i32;

    // 计算总时长（秒）
    let total_duration_seconds = summaries
        .iter()
        .map(|s| (s.end_time - s.start_time).num_seconds())
        .sum::<i64>();

    // 获取当前语言设置
    let current_language = {
        let lang_result = settings::load_language_from_db(&state.db_pool)
            .await
            .unwrap_or_else(|_| "zh".to_string());
        lang_result
    };

    // 获取对应语言的提示词
    let _prompt = settings::load_ai_prompt_from_db(&state.db_pool, Some(&current_language)).await
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
        let combined_content = summaries
            .iter()
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
            match video_summary::generate_text_summary_with_gemini(&key, &model, &daily_prompt)
                .await
            {
                Ok(summary_content) => summary_content,
                Err(e) => {
                    log::warn!(
                        "Failed to generate daily summary with AI: {}. Using combined summaries.",
                        e
                    );
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
pub async fn get_daily_summary(
    state: State<'_, AppState>,
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
#[tauri::command]
pub async fn get_historical_stats(
    state: State<'_, AppState>,
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
    let mut stats_map: std::collections::HashMap<String, HistoricalStats> =
        std::collections::HashMap::new();

    // 填充已有的每日总结数据
    for summary in daily_summaries {
        stats_map.insert(
            summary.date.clone(),
            HistoricalStats {
                date: summary.date.clone(),
                screenshot_count: summary.screenshot_count as i64,
                summary_count: summary.summary_count as i64,
                total_duration_seconds: summary.total_duration_seconds,
            },
        );
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
            let day_start = current_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "Invalid date".to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Invalid timezone conversion".to_string())?;

            let day_end = current_date
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| "Invalid date".to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Invalid timezone conversion".to_string())?;

            // 从数据库获取当天的截图和摘要
            let screenshots =
                db::get_screenshot_traces(&state.db_pool, Some(day_start), Some(day_end), None)
                    .await
                    .map_err(|e| format!("Database error: {}", e))?;

            let summaries = db::get_summaries(&state.db_pool, Some(day_start), Some(day_end), None)
                .await
                .map_err(|e| format!("Database error: {}", e))?;

            let total_duration = summaries
                .iter()
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
