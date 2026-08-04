//! 定时任务调度模块
//!
//! 在 WebUI 配置定时执行的任务组合，由后台调度器在指定时间自动执行。
//! 配置持久化到 JSON 文件（server/config/schedule.json）。
//!
//! cron 表达式格式（简化版）：
//!   "分 时 日 月 星期"  例如 "0 9 * * *" = 每天 09:00
//!
//! 防御性设计：调度器独立于核心任务运行；核心未加载时调度自动跳过并记录。

use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 一个定时任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// 唯一 ID
    pub id: String,
    /// 显示名
    pub name: String,
    /// cron 表达式（简化：分 时 日 月 星期）
    pub cron: String,
    /// 是否启用
    pub enabled: bool,
    /// 要执行的任务组合（与 /api/task 相同格式）
    pub tasks: Vec<Value>,
}

/// 调度器状态
pub struct Scheduler {
    config_path: PathBuf,
    tasks: Mutex<Vec<ScheduledTask>>,
}

impl Scheduler {
    pub fn new(config_path: PathBuf) -> Self {
        let tasks = Self::load_from_disk(&config_path).unwrap_or_default();
        Scheduler {
            config_path,
            tasks: Mutex::new(tasks),
        }
    }

    fn load_from_disk(path: &PathBuf) -> Result<Vec<ScheduledTask>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path)?;
        let list: Vec<ScheduledTask> = serde_json::from_str(&text)?;
        Ok(list)
    }

    /// 保存配置到磁盘
    async fn persist(&self, tasks: &[ScheduledTask]) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(tasks)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    /// 列出所有定时任务
    pub async fn list(&self) -> Vec<ScheduledTask> {
        self.tasks.lock().await.clone()
    }

    /// 添加定时任务
    pub async fn add(&self, task: ScheduledTask) -> Result<()> {
        let mut guard = self.tasks.lock().await;
        // 校验 cron 格式（简化）
        validate_cron(&task.cron)?;
        guard.push(task);
        let snapshot = guard.clone();
        self.persist(&snapshot).await?;
        Ok(())
    }

    /// 更新定时任务
    pub async fn update(&self, task: ScheduledTask) -> Result<()> {
        let mut guard = self.tasks.lock().await;
        validate_cron(&task.cron)?;
        if let Some(existing) = guard.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            guard.push(task);
        }
        let snapshot = guard.clone();
        self.persist(&snapshot).await?;
        Ok(())
    }

    /// 删除定时任务
    pub async fn remove(&self, id: &str) -> Result<()> {
        let mut guard = self.tasks.lock().await;
        guard.retain(|t| t.id != id);
        let snapshot = guard.clone();
        self.persist(&snapshot).await?;
        Ok(())
    }

    /// 获取当前应该触发的任务（基于当前时间匹配 cron）。
    /// 返回匹配的任务快照。
    pub async fn due_tasks(&self, now: chrono::DateTime<chrono::Local>) -> Vec<ScheduledTask> {
        let guard = self.tasks.lock().await;
        guard
            .iter()
            .filter(|t| t.enabled && cron_matches(&t.cron, now))
            .cloned()
            .collect()
    }
}

/// 解析 cron 简化表达式 "分 时 日 月 星期"，* 表示任意。
/// 返回 (minute, hour, day, month, weekday)，None 表示匹配任意。
fn parse_cron(cron: &str) -> Result<(Option<i32>, Option<i32>, Option<i32>, Option<i32>, Option<i32>)> {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(anyhow!("cron 表达式必须为 5 段：分 时 日 月 星期"));
    }
    fn parse_part(s: &str) -> Result<Option<i32>> {
        if s == "*" {
            Ok(None)
        } else {
            let v: i32 = s.parse().map_err(|_| anyhow!("无效的 cron 数值: {s}"))?;
            Ok(Some(v))
        }
    }
    Ok((
        parse_part(parts[0])?,
        parse_part(parts[1])?,
        parse_part(parts[2])?,
        parse_part(parts[3])?,
        parse_part(parts[4])?,
    ))
}

fn validate_cron(cron: &str) -> Result<()> {
    parse_cron(cron).map(|_| ())
}

/// 判断 cron 是否匹配当前时间
fn cron_matches(cron: &str, now: chrono::DateTime<chrono::Local>) -> bool {
    let parsed = match parse_cron(cron) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let (min, hour, day, month, weekday) = parsed;

    let now_min = now.minute() as i32;
    let now_hour = now.hour() as i32;
    let now_day = now.day() as i32;
    let now_month = now.month() as i32;
    let now_weekday = now.weekday().num_days_from_monday() as i32; // 0=周一

    if let Some(m) = min { if m != now_min { return false; } }
    if let Some(h) = hour { if h != now_hour { return false; } }
    if let Some(d) = day { if d != now_day { return false; } }
    if let Some(mo) = month { if mo != now_month { return false; } }
    if let Some(wd) = weekday { if wd != now_weekday { return false; } }
    true
}

/// 后台调度循环：每 30 秒检查一次是否有任务到期。
/// 通过回调执行任务组合（避免模块间循环依赖）。
pub async fn run_scheduler(
    scheduler: Arc<Scheduler>,
    execute: impl Fn(Vec<Value>) + Send + Sync + 'static,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        let now = chrono::Local::now();
        // 精确到分钟：只在 :00 秒检查
        if now.second() > 2 {
            continue;
        }
        let due = scheduler.due_tasks(now).await;
        for task in due {
            tracing::info!("定时任务触发: {} ({})", task.name, task.id);
            let tasks = task.tasks.clone();
            execute(tasks);
        }
    }
}
