use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotTrace {
    pub id: i64,
    pub timestamp: DateTime<Local>,
    pub file_path: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: i64,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub content: String,
    pub screenshot_count: i32,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    pub id: i64,
    pub date: String, // YYYY-MM-DD format
    pub content: String,
    pub screenshot_count: i32,
    pub summary_count: i32,
    pub total_duration_seconds: i64,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

// 获取数据库路径
fn get_db_path() -> PathBuf {
    let app_name = "clarity";

    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir()
            .map(|mut p| {
                p.push(app_name);
                p.push("clarity.db");
                p
            })
            .unwrap_or_else(|| {
                PathBuf::from(format!(
                    "C:\\Users\\{}\\AppData\\Local\\{}\\clarity.db",
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
                p.push("clarity.db");
                p
            })
            .unwrap_or_else(|| {
                PathBuf::from(format!(
                    "~/Library/Application Support/{}/clarity.db",
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
                p.push("clarity.db");
                p
            })
            .unwrap_or_else(|| PathBuf::from(format!("~/.local/share/{}/clarity.db", app_name)))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from(format!("./{}.db", app_name))
    }
}

// 初始化数据库连接
pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let db_path = get_db_path();

    // 确保目录存在
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 构建连接选项
    let connect_options =
        SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
            .create_if_missing(true);

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    // 创建表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS screenshot_traces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            file_path TEXT NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            file_size INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS summaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            content TEXT NOT NULL,
            screenshot_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // 创建索引以提高查询性能
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_traces_timestamp ON screenshot_traces(timestamp)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_summaries_start_time ON summaries(start_time)")
        .execute(&pool)
        .await?;

    // 创建 API 请求记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            model TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            total_tokens INTEGER,
            cost_usd REAL,
            status_code INTEGER,
            success INTEGER NOT NULL DEFAULT 1,
            error_message TEXT,
            request_duration_ms INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_requests_timestamp ON api_requests(timestamp)")
        .execute(&pool)
        .await?;

    // 创建每日总结表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            screenshot_count INTEGER NOT NULL DEFAULT 0,
            summary_count INTEGER NOT NULL DEFAULT 0,
            total_duration_seconds INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_daily_summaries_date ON daily_summaries(date)")
        .execute(&pool)
        .await?;

    Ok(pool)
}

// 插入截图记录
pub async fn insert_screenshot_trace(
    pool: &SqlitePool,
    timestamp: DateTime<Local>,
    file_path: String,
    width: i32,
    height: i32,
    file_size: i64,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query(
        r#"
        INSERT INTO screenshot_traces (timestamp, file_path, width, height, file_size)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(timestamp.to_rfc3339())
    .bind(file_path)
    .bind(width)
    .bind(height)
    .bind(file_size)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

// 查询截图记录（按时间范围）
pub async fn get_screenshot_traces(
    pool: &SqlitePool,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
    limit: Option<i64>,
) -> Result<Vec<ScreenshotTrace>, sqlx::Error> {
    let mut query = String::from("SELECT id, timestamp, file_path, width, height, file_size FROM screenshot_traces WHERE 1=1");
    let mut conditions = Vec::new();

    if let Some(start) = start_time {
        conditions.push(format!("timestamp >= '{}'", start.to_rfc3339()));
    }
    if let Some(end) = end_time {
        conditions.push(format!("timestamp <= '{}'", end.to_rfc3339()));
    }

    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY timestamp DESC");

    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }

    let rows = sqlx::query(&query).fetch_all(pool).await?;

    let mut traces = Vec::new();
    for row in rows {
        let timestamp_str: String = row.get(1);
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|_| sqlx::Error::Decode("Invalid timestamp format".into()))?
            .with_timezone(&Local);

        traces.push(ScreenshotTrace {
            id: row.get(0),
            timestamp,
            file_path: row.get(2),
            width: row.get(3),
            height: row.get(4),
            file_size: row.get(5),
        });
    }

    Ok(traces)
}

// 插入摘要
pub async fn insert_summary(
    pool: &SqlitePool,
    start_time: DateTime<Local>,
    end_time: DateTime<Local>,
    content: String,
    screenshot_count: i32,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query(
        r#"
        INSERT INTO summaries (start_time, end_time, content, screenshot_count)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(start_time.to_rfc3339())
    .bind(end_time.to_rfc3339())
    .bind(content)
    .bind(screenshot_count)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

// 查询摘要（按时间范围）
pub async fn get_summaries(
    pool: &SqlitePool,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
    limit: Option<i64>,
) -> Result<Vec<Summary>, sqlx::Error> {
    let mut query = String::from("SELECT id, start_time, end_time, content, screenshot_count, created_at FROM summaries WHERE 1=1");
    let mut conditions = Vec::new();

    if let Some(start) = start_time {
        conditions.push(format!("start_time >= '{}'", start.to_rfc3339()));
    }
    if let Some(end) = end_time {
        conditions.push(format!("end_time <= '{}'", end.to_rfc3339()));
    }

    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY start_time DESC");

    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }

    let rows = sqlx::query(&query).fetch_all(pool).await?;

    let mut summaries = Vec::new();
    for row in rows {
        let start_time_str: String = row.get(1);
        let end_time_str: String = row.get(2);
        let created_at_str: String = row.get(5);

        // 尝试解析 RFC3339 格式，如果失败则尝试 SQLite 格式
        let start_time = parse_timestamp(&start_time_str)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid start_time format: {}", e).into()))?;

        let end_time = parse_timestamp(&end_time_str)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid end_time format: {}", e).into()))?;

        let created_at = parse_timestamp(&created_at_str)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid created_at format: {}", e).into()))?;

        summaries.push(Summary {
            id: row.get(0),
            start_time,
            end_time,
            content: row.get(3),
            screenshot_count: row.get(4),
            created_at,
        });
    }

    Ok(summaries)
}

// API 请求记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequest {
    pub id: i64,
    pub timestamp: DateTime<Local>,
    pub model: String,
    pub endpoint: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub request_duration_ms: Option<i64>,
}

// 插入 API 请求记录
pub async fn insert_api_request(
    pool: &SqlitePool,
    model: &str,
    endpoint: &str,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
    status_code: u16,
    success: bool,
    error_message: Option<&str>,
    duration_ms: u64,
) -> Result<i64, sqlx::Error> {
    use chrono::Local;

    let timestamp = Local::now().to_rfc3339();

    let id = sqlx::query(
        r#"
        INSERT INTO api_requests (
            timestamp, model, endpoint, prompt_tokens, completion_tokens, total_tokens,
            status_code, success, error_message, request_duration_ms
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(timestamp)
    .bind(model)
    .bind(endpoint)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(status_code as i32)
    .bind(if success { 1 } else { 0 })
    .bind(error_message)
    .bind(duration_ms as i64)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

// 获取 API 请求统计
pub async fn get_api_statistics(
    pool: &SqlitePool,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
) -> Result<ApiStatistics, sqlx::Error> {
    let mut query = String::from(
        "SELECT 
            COALESCE(COUNT(*), 0) as total_requests,
            COALESCE(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END), 0) as successful_requests,
            COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0) as failed_requests,
            COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
            COALESCE(SUM(total_tokens), 0) as total_tokens,
            AVG(request_duration_ms) as avg_duration_ms
        FROM api_requests WHERE 1=1",
    );

    if let Some(start) = start_time {
        query.push_str(&format!(" AND timestamp >= '{}'", start.to_rfc3339()));
    }
    if let Some(end) = end_time {
        query.push_str(&format!(" AND timestamp <= '{}'", end.to_rfc3339()));
    }

    let row = sqlx::query(&query).fetch_one(pool).await?;

    Ok(ApiStatistics {
        total_requests: row.get::<i64, _>(0),
        successful_requests: row.get::<i64, _>(1),
        failed_requests: row.get::<i64, _>(2),
        total_prompt_tokens: row.get::<i64, _>(3),
        total_completion_tokens: row.get::<i64, _>(4),
        total_tokens: row.get::<i64, _>(5),
        avg_duration_ms: row.get::<Option<f64>, _>(6),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStatistics {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub avg_duration_ms: Option<f64>,
}

// 解析时间戳，支持多种格式
fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Local>, String> {
    // 首先尝试 RFC3339 格式
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        return Ok(dt.with_timezone(&Local));
    }

    // 尝试 SQLite 的 datetime 格式: "YYYY-MM-DD HH:MM:SS"
    if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt
            .and_local_timezone(Local)
            .single()
            .ok_or_else(|| "Invalid timezone conversion".to_string())?);
    }

    // 尝试带毫秒的格式: "YYYY-MM-DD HH:MM:SS.fff"
    if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(dt
            .and_local_timezone(Local)
            .single()
            .ok_or_else(|| "Invalid timezone conversion".to_string())?);
    }

    Err(format!("Unable to parse timestamp: {}", timestamp_str))
}

// 获取今天的截图数量
pub async fn get_today_screenshot_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM screenshot_traces WHERE date(timestamp) = date('now')",
    )
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

// 插入或更新每日总结
pub async fn upsert_daily_summary(
    pool: &SqlitePool,
    date: &str, // YYYY-MM-DD format
    content: &str,
    screenshot_count: i32,
    summary_count: i32,
    total_duration_seconds: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO daily_summaries (date, content, screenshot_count, summary_count, total_duration_seconds, updated_at)
        VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(date) DO UPDATE SET
            content = excluded.content,
            screenshot_count = excluded.screenshot_count,
            summary_count = excluded.summary_count,
            total_duration_seconds = excluded.total_duration_seconds,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(date)
    .bind(content)
    .bind(screenshot_count)
    .bind(summary_count)
    .bind(total_duration_seconds)
    .execute(pool)
    .await?;

    // 获取插入或更新的 ID
    let result: Option<(i64,)> = sqlx::query_as("SELECT id FROM daily_summaries WHERE date = ?")
        .bind(date)
        .fetch_optional(pool)
        .await?;

    Ok(result.map(|r| r.0).unwrap_or(0))
}

// 获取每日总结
pub async fn get_daily_summary(
    pool: &SqlitePool,
    date: &str, // YYYY-MM-DD format
) -> Result<Option<DailySummary>, sqlx::Error> {
    let result: Option<(i64, String, String, i32, i32, i64, String, String)> = sqlx::query_as(
        "SELECT id, date, content, screenshot_count, summary_count, total_duration_seconds, created_at, updated_at FROM daily_summaries WHERE date = ?"
    )
    .bind(date)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = result {
        let created_at = parse_timestamp(&row.6)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid created_at format: {}", e).into()))?;
        let updated_at = parse_timestamp(&row.7)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid updated_at format: {}", e).into()))?;

        Ok(Some(DailySummary {
            id: row.0,
            date: row.1,
            content: row.2,
            screenshot_count: row.3,
            summary_count: row.4,
            total_duration_seconds: row.5,
            created_at,
            updated_at,
        }))
    } else {
        Ok(None)
    }
}

// 获取日期范围内的每日总结
pub async fn get_daily_summaries(
    pool: &SqlitePool,
    start_date: Option<&str>, // YYYY-MM-DD format
    end_date: Option<&str>,   // YYYY-MM-DD format
    limit: Option<i64>,
) -> Result<Vec<DailySummary>, sqlx::Error> {
    let mut query = String::from("SELECT id, date, content, screenshot_count, summary_count, total_duration_seconds, created_at, updated_at FROM daily_summaries WHERE 1=1");
    let mut conditions = Vec::new();

    if let Some(start) = start_date {
        conditions.push(format!("date >= '{}'", start));
    }
    if let Some(end) = end_date {
        conditions.push(format!("date <= '{}'", end));
    }

    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY date DESC");

    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }

    let rows = sqlx::query(&query).fetch_all(pool).await?;

    let mut summaries = Vec::new();
    for row in rows {
        let created_at_str: String = row.get(6);
        let updated_at_str: String = row.get(7);

        let created_at = parse_timestamp(&created_at_str)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid created_at format: {}", e).into()))?;
        let updated_at = parse_timestamp(&updated_at_str)
            .map_err(|e| sqlx::Error::Decode(format!("Invalid updated_at format: {}", e).into()))?;

        summaries.push(DailySummary {
            id: row.get(0),
            date: row.get(1),
            content: row.get(2),
            screenshot_count: row.get(3),
            summary_count: row.get(4),
            total_duration_seconds: row.get(5),
            created_at,
            updated_at,
        });
    }

    Ok(summaries)
}
