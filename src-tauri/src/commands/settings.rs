use crate::db;
use crate::settings;
use crate::state::AppState;
use tauri::State;

// 获取 Google Gemini API Key
#[tauri::command]
pub async fn get_gemini_api_key(state: State<'_, AppState>) -> Result<String, String> {
    let api_key = state.gemini_api_key.lock().await.clone();
    Ok(api_key.unwrap_or_default())
}

// 设置 Google Gemini API Key
#[tauri::command]
pub async fn set_gemini_api_key(state: State<'_, AppState>, api_key: String) -> Result<(), String> {
    // 保存到数据库
    settings::save_api_key_to_db(&state.db_pool, &api_key)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 更新内存中的值
    *state.gemini_api_key.lock().await = Some(api_key);

    Ok(())
}

// 获取总结间隔（秒）
#[tauri::command]
pub async fn get_summary_interval(state: State<'_, AppState>) -> Result<u64, String> {
    let interval = *state.summary_interval_seconds.lock().await;
    log::info!("Getting summary interval: {} seconds", interval);
    Ok(interval)
}

// 设置总结间隔（秒）
#[tauri::command]
pub async fn set_summary_interval(
    state: State<'_, AppState>,
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
    settings::save_summary_interval_to_db(&state.db_pool, interval_seconds)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 更新内存中的值
    *state.summary_interval_seconds.lock().await = interval_seconds;
    log::info!("Summary interval updated successfully");

    Ok(())
}

// 测试视频总结功能（诊断用）
#[tauri::command]
pub async fn test_video_summary(state: State<'_, AppState>) -> Result<String, String> {
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
        vec![
            "ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/opt/homebrew/bin/ffmpeg",
        ]
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
    let count = db::get_today_screenshot_count(&state.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    diagnostics.push(format!("📸 Today's screenshots: {}", count));

    // 检查总结间隔
    let interval = *state.summary_interval_seconds.lock().await;
    diagnostics.push(format!("⏱️ Summary interval: {} seconds", interval));

    // 检查是否在录制
    let recording = *state.is_recording.lock().await;
    diagnostics.push(format!(
        "🎬 Recording: {}",
        if recording { "Yes" } else { "No" }
    ));

    // 检查存储路径
    let storage_path = state.storage_path.lock().await.clone();
    diagnostics.push(format!("📁 Storage path: {}", storage_path.display()));

    let result = diagnostics.join("\n");
    log::info!("Video summary diagnostics:\n{}", result);
    Ok(result)
}

// 获取 AI 模型
#[tauri::command]
pub async fn get_ai_model(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.ai_model.lock().await.clone())
}

// 设置 AI 模型
#[tauri::command]
pub async fn set_ai_model(state: State<'_, AppState>, model: String) -> Result<(), String> {
    if model.is_empty() {
        return Err("Model cannot be empty".to_string());
    }

    // 保存到数据库
    settings::save_ai_model_to_db(&state.db_pool, &model)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 更新内存中的值
    *state.ai_model.lock().await = model;
    log::info!("AI model updated successfully");

    Ok(())
}

// 获取 AI 提示词（按语言）
#[tauri::command]
pub async fn get_ai_prompt(
    state: State<'_, AppState>,
    language: Option<String>,
) -> Result<String, String> {
    let lang = language.as_deref().unwrap_or("zh");

    // 从数据库加载指定语言的提示词
    match settings::load_ai_prompt_from_db(&state.db_pool, Some(lang)).await {
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
pub async fn set_ai_prompt(
    state: State<'_, AppState>,
    prompt: String,
    language: Option<String>,
) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    let lang = language.as_deref().unwrap_or("zh");

    // 保存到数据库（按语言）
    settings::save_ai_prompt_to_db(&state.db_pool, &prompt, Some(lang))
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
pub async fn reset_ai_prompt(
    state: State<'_, AppState>,
    language: Option<String>,
) -> Result<String, String> {
    let lang = language.as_deref().unwrap_or("zh");

    let default_prompt = if lang == "en" {
        "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string()
    } else {
        "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string()
    };

    // 保存到数据库（按语言）
    settings::save_ai_prompt_to_db(&state.db_pool, &default_prompt, Some(lang))
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    log::info!("AI prompt reset to default for language: {}", lang);

    Ok(default_prompt)
}

// 获取视频分辨率设置
#[tauri::command]
pub async fn get_video_resolution(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.video_resolution.lock().await.clone())
}

// 设置视频分辨率
#[tauri::command]
pub async fn set_video_resolution(
    state: State<'_, AppState>,
    resolution: String,
) -> Result<(), String> {
    if resolution != "low" && resolution != "default" {
        return Err("Resolution must be 'low' or 'default'".to_string());
    }

    // 保存到数据库
    settings::save_video_resolution_to_db(&state.db_pool, &resolution)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 更新内存中的值
    *state.video_resolution.lock().await = resolution.clone();
    log::info!("Video resolution updated to: {}", resolution);

    Ok(())
}

// 获取语言设置
#[tauri::command]
pub async fn get_language(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.language.lock().await.clone())
}

// 设置语言
#[tauri::command]
pub async fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    if language != "en" && language != "zh" {
        return Err("Language must be 'en' or 'zh'".to_string());
    }

    // 保存到数据库
    settings::save_language_to_db(&state.db_pool, &language)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // 更新内存中的值
    *state.language.lock().await = language.clone();
    log::info!("Language updated to: {}", language);

    Ok(())
}
