use crate::db;
use crate::screenshot;
use crate::settings;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

// 全局状态管理
pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,
    pub screenshots_count: Arc<Mutex<u64>>,
    pub storage_path: Arc<Mutex<PathBuf>>,
    pub handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub db_pool: SqlitePool,
    pub gemini_api_key: Arc<Mutex<Option<String>>>,
    pub summary_interval_seconds: Arc<Mutex<u64>>,
    pub app_handle: Arc<Mutex<Option<AppHandle>>>,
    pub ai_model: Arc<Mutex<String>>,
    pub _ai_prompt: Arc<Mutex<String>>,
    pub language: Arc<Mutex<String>>,
    pub video_resolution: Arc<Mutex<String>>, // "low" or "default"
}

impl AppState {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let db_pool = db::init_db().await?;

        // 从数据库加载 API key
        let api_key = settings::load_api_key_from_db(&db_pool).await.ok();

        // 从数据库加载总结间隔（默认 45 秒）
        let summary_interval = settings::load_summary_interval_from_db(&db_pool)
            .await
            .unwrap_or(45);

        // 从数据库加载 AI 模型（默认 gemini-3-flash-preview）
        let ai_model = settings::load_ai_model_from_db(&db_pool)
            .await
            .unwrap_or_else(|_| "gemini-3-flash-preview".to_string());

        // 从数据库加载视频分辨率设置（默认 low，节省 token）
        let video_resolution = settings::load_video_resolution_from_db(&db_pool)
            .await
            .unwrap_or_else(|_| "low".to_string());

        // 从数据库加载 AI 提示词（默认根据系统语言，如果没有则使用中文）
        // 优化后的 prompt：更聚焦于效率分析，减少不必要的描述
        let default_prompt_zh = "分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。".to_string();
        let _default_prompt_en = "Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.".to_string();

        // 尝试加载中文提示词，如果没有则使用默认值
        let ai_prompt = settings::load_ai_prompt_from_db(&db_pool, Some("zh"))
            .await
            .unwrap_or_else(|_| default_prompt_zh.clone());

        // 从数据库加载语言设置（默认中文）
        let language = settings::load_language_from_db(&db_pool)
            .await
            .unwrap_or_else(|_| "zh".to_string());

        Ok(Self {
            is_recording: Arc::new(Mutex::new(false)),
            screenshots_count: Arc::new(Mutex::new(0)),
            storage_path: Arc::new(Mutex::new(screenshot::get_app_data_dir())),
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
    pub async fn emit_statistics_updated(&self) {
        if let Some(handle) = self.app_handle.lock().await.as_ref() {
            let _ = handle.emit("statistics-updated", ());
        }
    }
}
