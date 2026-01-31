use sqlx::SqlitePool;

// 从数据库加载 API key
pub async fn load_api_key_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'gemini_api_key' LIMIT 1")
            .fetch_optional(pool)
            .await?;

    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存 API key 到数据库
pub async fn save_api_key_to_db(pool: &SqlitePool, api_key: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 插入或更新
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('gemini_api_key', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(api_key)
    .execute(pool)
    .await?;

    Ok(())
}

// 从数据库加载视频分辨率设置
pub async fn load_video_resolution_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'video_resolution' LIMIT 1")
            .fetch_optional(pool)
            .await?;

    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存视频分辨率设置到数据库
pub async fn save_video_resolution_to_db(
    pool: &SqlitePool,
    resolution: &str,
) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('video_resolution', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(resolution)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载 AI 模型
pub async fn load_ai_model_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'ai_model' LIMIT 1")
            .fetch_optional(pool)
            .await?;

    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 保存 AI 模型到数据库
pub async fn save_ai_model_to_db(pool: &SqlitePool, model: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('ai_model', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(model)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载语言设置
pub async fn load_language_from_db(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'language' LIMIT 1")
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
pub async fn save_language_to_db(pool: &SqlitePool, language: &str) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('language', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(language)
    .execute(pool)
    .await?;
    Ok(())
}

// 保存 AI 提示词到数据库（按语言）
pub async fn save_ai_prompt_to_db(
    pool: &SqlitePool,
    prompt: &str,
    language: Option<&str>,
) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    let key = match language {
        Some("zh") => "ai_prompt_zh",
        Some("en") => "ai_prompt_en",
        _ => "ai_prompt", // 默认兼容旧版本
    };

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(key)
    .bind(prompt)
    .execute(pool)
    .await?;
    Ok(())
}

// 从数据库加载 AI 提示词（按语言）
pub async fn load_ai_prompt_from_db(
    pool: &SqlitePool,
    language: Option<&str>,
) -> Result<String, sqlx::Error> {
    let key = match language {
        Some("zh") => "ai_prompt_zh",
        Some("en") => "ai_prompt_en",
        _ => "ai_prompt", // 默认兼容旧版本
    };

    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?1 LIMIT 1")
            .bind(key)
            .fetch_optional(pool)
            .await?;

    result.map(|r| r.0).ok_or_else(|| sqlx::Error::RowNotFound)
}

// 从数据库加载总结间隔
pub async fn load_summary_interval_from_db(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'summary_interval_seconds' LIMIT 1")
            .fetch_optional(pool)
            .await?;

    if let Some((value,)) = result {
        value
            .parse::<u64>()
            .map_err(|_| sqlx::Error::Decode("Invalid summary interval format".into()))
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

// 保存总结间隔到数据库
pub async fn save_summary_interval_to_db(
    pool: &SqlitePool,
    interval_seconds: u64,
) -> Result<(), sqlx::Error> {
    // 确保 settings 表存在
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 插入或更新
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('summary_interval_seconds', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(interval_seconds.to_string())
    .execute(pool)
    .await?;

    Ok(())
}
