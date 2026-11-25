//! TdPrint 发行版：日志与诊断导出。
//!
//! - 日志：INFO 级别（本项目 crate 为 DEBUG），单文件 1 MB、另保留 3 个归档，总量不超过约 4 MB。
//!   上游默认 TRACE + 单文件 40 KB，WebSocket 帧日志几分钟就把有用信息挤掉。
//! - 诊断导出：任务页「下载日志」把运行环境、最近任务历史和应用日志写成一个 txt，
//!   仓库人员发给开发分析打印失败原因；导出内容有上限（日志最多 2 MB、任务最多 300 条），
//!   不导出远程任务 Token 等凭据。
//!
//! 定制逻辑集中在本文件，上游 lib.rs 只保留调用点，便于合并上游更新。

use crate::{config::AgentConfig, state::AgentState, task_history::TaskHistoryStore};
use print_bridge_runtime::tdprint_auth;
use serde::Serialize;
use std::{
    fmt::Write as _,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};
use tauri::{plugin::TauriPlugin, AppHandle, Manager, Runtime, State};
use tauri_plugin_log::{log::LevelFilter, RotationStrategy, TimezoneStrategy};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// 单个日志文件上限。
const LOG_MAX_FILE_BYTES: u128 = 1024 * 1024;
/// 归档日志保留个数（另有 1 个当前文件）。
const LOG_KEEP_ARCHIVES: usize = 3;
/// 导出时最多附带的日志字节数（取最新的部分）。
const EXPORT_MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
/// 导出时最多列出的任务数。
const EXPORT_MAX_JOBS: u32 = 300;
/// 导出时附带状态明细的最近任务数。
const EXPORT_EVENT_JOBS: usize = 100;

/// TdPrint 的日志插件配置，替代上游 `tauri_plugin_log::Builder::new().build()`。
pub fn log_plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .level(LevelFilter::Info)
        .level_for("print_bridge_lib", LevelFilter::Debug)
        .level_for("print_bridge_runtime", LevelFilter::Debug)
        // 每次 HTML 渲染都会打印浏览器进程启停，只保留告警
        .level_for("headless_chrome", LevelFilter::Warn)
        .max_file_size(LOG_MAX_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(LOG_KEEP_ARCHIVES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

/// 把诊断信息写到用户选择的文件，返回写入字节数。
#[tauri::command]
pub async fn tdprint_export_diagnostics(
    path: String,
    app: AppHandle,
    state: State<'_, AgentState>,
) -> Result<u64, String> {
    let config = state.config.read().await.clone();
    let history = state.task_history.clone();
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|error| error.to_string())?;
    let version = app.package_info().version.to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let report = build_report(&version, &config, history.as_deref(), &log_dir);
        fs::write(&path, report.as_bytes())
            .map_err(|error| format!("写入诊断日志失败：{error}"))?;
        Ok(report.len() as u64)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn build_report(
    version: &str,
    config: &AgentConfig,
    history: Option<&TaskHistoryStore>,
    log_dir: &Path,
) -> String {
    let mut out = String::new();
    write_environment(&mut out, version, config);
    write_task_history(&mut out, history);
    write_logs(&mut out, log_dir);
    out
}

fn write_environment(out: &mut String, version: &str, config: &AgentConfig) {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let paper = config
        .printing
        .default_paper
        .as_ref()
        .map(|paper| format!("{} x {} mm", paper.width_mm, paper.height_mm));
    let remote = &config.remote;

    let _ = writeln!(out, "TdPrint 诊断日志");
    let _ = writeln!(out, "==============================");
    let _ = writeln!(out, "导出时间(UTC): {now}");
    let _ = writeln!(out, "版本: {version}");
    let _ = writeln!(
        out,
        "系统: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(out, "本地端口: {}", config.service.port);
    let _ = writeln!(
        out,
        "内置令牌: {}",
        if tdprint_auth::is_enabled() {
            "已启用"
        } else {
            "未启用"
        }
    );
    let _ = writeln!(
        out,
        "内置网站白名单: {}",
        tdprint_auth::BUILTIN_ORIGINS.join(", ")
    );
    let _ = writeln!(
        out,
        "网站白名单: {}",
        or_none(&config.security.allowed_origins.join(", "))
    );
    let _ = writeln!(
        out,
        "IP 白名单: {}",
        or_none(&config.security.allowed_ips.join(", "))
    );
    let _ = writeln!(
        out,
        "默认打印机: {}",
        config
            .printing
            .default_printer
            .as_deref()
            .unwrap_or("（未设置）")
    );
    let _ = writeln!(
        out,
        "默认纸张: {}",
        paper.as_deref().unwrap_or("（未设置）")
    );
    let _ = writeln!(out, "默认份数: {}", config.printing.default_copies);
    let _ = writeln!(
        out,
        "限制: 文件 {} MB，批量 {} 个",
        config.limits.max_file_size_mb, config.limits.max_batch_jobs
    );
    // 只导出远程任务是否启用和地址，不导出 Token
    let _ = writeln!(
        out,
        "远程任务: {}{}",
        if remote.enabled { "启用" } else { "关闭" },
        remote
            .endpoint_url
            .as_deref()
            .map(|url| format!("（{url}）"))
            .unwrap_or_default()
    );
    let _ = writeln!(out);
}

fn write_task_history(out: &mut String, history: Option<&TaskHistoryStore>) {
    let _ = writeln!(
        out,
        "===== 最近任务（最多 {EXPORT_MAX_JOBS} 条，最新在前，时间为 UTC）====="
    );
    let Some(history) = history else {
        let _ = writeln!(out, "（任务历史不可用）\n");
        return;
    };
    let jobs = match history.recent_jobs(EXPORT_MAX_JOBS) {
        Ok(jobs) => jobs,
        Err(error) => {
            let _ = writeln!(out, "（读取任务历史失败：{error}）\n");
            return;
        }
    };
    if jobs.is_empty() {
        let _ = writeln!(out, "（暂无任务）");
    }
    for (index, job) in jobs.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}  {:<10} {:<10} {}  打印机={}  纸张={}  {}",
            job.created_at,
            enum_text(&job.current_status),
            enum_text(&job.source),
            job.job_id,
            job.printer_name.as_deref().unwrap_or("-"),
            job.paper_name.as_deref().unwrap_or("-"),
            job.current_message.as_deref().unwrap_or("")
        );
        if index >= EXPORT_EVENT_JOBS {
            continue;
        }
        if let Ok(events) = history.events_for_job(&job.job_id) {
            for event in events {
                let _ = writeln!(
                    out,
                    "    {}  {:<10} {}",
                    event.occurred_at,
                    enum_text(&event.status),
                    event.message.as_deref().unwrap_or("")
                );
            }
        }
    }
    let _ = writeln!(out);
}

fn write_logs(out: &mut String, log_dir: &Path) {
    let _ = writeln!(
        out,
        "===== 应用日志（最新 {} MB，时间为本机时间）=====",
        EXPORT_MAX_LOG_BYTES / 1024 / 1024
    );
    let files = log_files_newest_first(log_dir);
    if files.is_empty() {
        let _ = writeln!(out, "（暂无日志：{}）", log_dir.display());
        return;
    }

    // 从最新文件往前取，直到用完字节预算，再按时间顺序输出
    let mut budget = EXPORT_MAX_LOG_BYTES;
    let mut chunks = Vec::new();
    for path in files {
        if budget == 0 {
            break;
        }
        match read_tail(&path, budget) {
            Ok((text, read)) => {
                budget = budget.saturating_sub(read);
                chunks.push((path, text));
            }
            Err(error) => chunks.push((path, format!("（读取失败：{error}）\n"))),
        }
    }
    for (path, text) in chunks.into_iter().rev() {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let _ = writeln!(out, "--- {name} ---");
        out.push_str(&text);
        if !text.ends_with('\n') {
            out.push('\n');
        }
    }
}

/// 日志目录下的 .log 文件，按修改时间从新到旧。
fn log_files_newest_first(log_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return Vec::new();
    };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok()?;
            Some((modified, path))
        })
        .collect();
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    files.into_iter().map(|(_, path)| path).collect()
}

/// 读取文件末尾最多 `max_bytes` 字节，返回文本与实际读取字节数。
fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = Vec::with_capacity((len - start) as usize);
    file.read_to_end(&mut buffer)?;
    let read = buffer.len() as u64;
    let mut text = String::from_utf8_lossy(&buffer).into_owned();
    if start > 0 {
        // 截断处可能在一行中间，丢掉不完整的首行
        if let Some(pos) = text.find('\n') {
            text.drain(..=pos);
        }
        text.insert_str(0, "（前面内容已省略）\n");
    }
    Ok((text, read))
}

/// serde snake_case 名称（如 failed / web_socket），失败时退回 Debug。
fn enum_text<T: Serialize + std::fmt::Debug>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{value:?}"))
}

fn or_none(value: &str) -> &str {
    if value.is_empty() {
        "（无）"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_history::{NewTaskHistoryEvent, TaskHistorySource, TaskHistoryStatus};

    #[test]
    fn report_contains_environment_tasks_and_log_tail_without_remote_token() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::write(log_dir.join("TdPrint.log"), "line-1\nline-2\n").unwrap();

        let mut config = AgentConfig::default();
        config.printing.default_printer = Some("Brother DCP-T725DW Printer".to_string());
        config.remote.bearer_token = Some("remote-secret".to_string());

        let history = TaskHistoryStore::open_in_memory().unwrap();
        history
            .record_event(&NewTaskHistoryEvent {
                job_id: "JOB-1",
                request_id: Some("REQ-1"),
                batch_id: None,
                source: TaskHistorySource::WebSocket,
                status: TaskHistoryStatus::Failed,
                message: Some("printer not configured"),
                printer_name: None,
                paper_name: None,
                copies: None,
                occurred_at: "2026-11-25T03:55:17Z",
            })
            .unwrap();

        let report = build_report("0.2.9", &config, Some(&history), &log_dir);

        assert!(report.contains("版本: 0.2.9"));
        assert!(report.contains("默认打印机: Brother DCP-T725DW Printer"));
        assert!(report.contains("JOB-1"));
        assert!(report.contains("failed"));
        assert!(report.contains("printer not configured"));
        assert!(report.contains("--- TdPrint.log ---\nline-1\nline-2\n"));
        assert!(!report.contains("remote-secret"));
    }

    #[test]
    fn read_tail_keeps_only_the_newest_bytes_from_a_line_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.log");
        fs::write(&path, "aaaa\nbbbb\ncccc\n").unwrap();

        let (text, read) = read_tail(&path, 7).unwrap();

        assert_eq!(read, 7);
        assert_eq!(text, "（前面内容已省略）\ncccc\n");
    }
}
