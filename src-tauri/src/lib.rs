mod commands;
mod db;
mod screenshot;
mod settings;
mod state;
mod video_summary;

use state::AppState;
use tauri::Manager;

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
                let app_state = AppState::new().await.map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to initialize database: {}",
                        e
                    ))
                })?;

                // 保存 app handle 用于发送事件
                *app_state.app_handle.lock().await = Some(app.handle().clone());

                log::info!("Application state initialized successfully");
                app.manage(app_state);
                Ok(())
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::get_status,
            commands::get_storage_path,
            commands::test_screenshot,
            commands::get_traces,
            commands::get_summaries,
            commands::add_summary,
            commands::get_today_count,
            commands::get_gemini_api_key,
            commands::set_gemini_api_key,
            commands::get_summary_interval,
            commands::set_summary_interval,
            commands::test_video_summary,
            commands::get_api_statistics,
            commands::get_today_statistics,
            commands::get_ai_model,
            commands::set_ai_model,
            commands::get_ai_prompt,
            commands::set_ai_prompt,
            commands::reset_ai_prompt,
            commands::get_language,
            commands::set_language,
            commands::generate_daily_summary,
            commands::get_daily_summary,
            commands::get_historical_stats,
            commands::get_video_resolution,
            commands::set_video_resolution,
            commands::read_screenshot_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
