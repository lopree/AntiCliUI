use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ConversationItem {
    pub id: String,
    pub title: String,
    pub last_modified: String,
    pub step_count: i64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TranscriptType {
    User,
    Assistant,
    Thinking,
    ToolCall { name: String, args: String },
    ToolResult { output: String },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TranscriptEntry {
    pub step_index: usize,
    pub entry_type: TranscriptType,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QuotaInfo {
    pub context_tokens: u64,
    pub context_max: u64,
    pub used_pct: f64,

    pub five_hour_pct: f64,
    pub five_hour_reset: String,

    pub weekly_pct: f64,
    pub weekly_reset: String,

    pub active_model: String,
}

impl Default for QuotaInfo {
    fn default() -> Self {
        Self {
            context_tokens: 227_473,
            context_max: 1_048_576,
            used_pct: 21.7,
            five_hour_pct: 88.5,
            five_hour_reset: "3小时8分".to_string(),
            weekly_pct: 91.2,
            weekly_reset: "4天18小时".to_string(),
            active_model: "Gemini 3.8 Flash (High)".to_string(),
        }
    }
}

pub fn base_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home/rookstein"))
        .join(".gemini/antigravity-cli")
}

pub fn db_path() -> PathBuf {
    base_dir().join("conversation_summaries.db")
}

pub fn deleted_file() -> PathBuf {
    base_dir().join("deleted_conversations.txt")
}

pub fn settings_path() -> PathBuf {
    base_dir().join("settings.json")
}

pub fn get_deleted_ids() -> HashSet<String> {
    let p = deleted_file();
    if !p.exists() {
        return HashSet::new();
    }
    if let Ok(content) = fs::read_to_string(p) {
        content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        HashSet::new()
    }
}

pub fn load_sessions() -> Result<Vec<ConversationItem>> {
    let db = db_path();
    if !db.exists() {
        return Ok(vec![]);
    }
    let deleted = get_deleted_ids();
    let conn = Connection::open(&db)?;

    let mut stmt = conn.prepare(
        "SELECT conversation_id, title, datetime(last_modified_time, 'localtime'), step_count
         FROM conversation_summaries
         WHERE (nesting_depth = 0 OR nesting_depth IS NULL)
           AND (parent_conversation_id = '' OR parent_conversation_id IS NULL)
           AND (agent_name = '' OR agent_name IS NULL)
           AND (step_count > 0)
         ORDER BY last_modified_time DESC
         LIMIT 100",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ConversationItem {
            id: row.get(0)?,
            title: row.get(1)?,
            last_modified: row.get(2)?,
            step_count: row.get(3)?,
        })
    })?;

    let mut result = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            if !deleted.contains(&item.id) {
                result.push(item);
            }
        }
    }
    Ok(result)
}

pub fn format_duration(secs: u64) -> String {
    if secs == 0 {
        return "即将重置".to_string();
    }
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        if hours > 0 {
            format!("{}天{}小时", days, hours)
        } else {
            format!("{}天", days)
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{}小时{}分", hours, mins)
        } else {
            format!("{}小时", hours)
        }
    } else {
        format!("{}分", mins)
    }
}

pub fn load_quota_info() -> QuotaInfo {
    let debug_file = PathBuf::from("/tmp/antigravity_statusline_stdin.json");
    if debug_file.exists() {
        if let Ok(content) = fs::read_to_string(&debug_file) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                let context = &val["context_window"];
                let tokens = context["total_input_tokens"].as_u64().unwrap_or(227473);
                let max_tokens = context["context_window_size"].as_u64().unwrap_or(1048576);
                let used_pct = context["used_percentage"].as_f64().unwrap_or(21.7);

                let quota = &val["quota"];
                let five_h_bucket = if quota["gemini-5h"].is_object() {
                    &quota["gemini-5h"]
                } else {
                    &quota["3p-5h"]
                };
                let weekly_bucket = if quota["gemini-weekly"].is_object() {
                    &quota["gemini-weekly"]
                } else {
                    &quota["3p-weekly"]
                };

                let five_h_rem = five_h_bucket["remaining_fraction"].as_f64().unwrap_or(0.885) * 100.0;
                let five_h_secs = five_h_bucket["reset_in_seconds"].as_u64().unwrap_or(11308);

                let weekly_rem = weekly_bucket["remaining_fraction"].as_f64().unwrap_or(0.912) * 100.0;
                let weekly_secs = weekly_bucket["reset_in_seconds"].as_u64().unwrap_or(412935);

                let model = val["model"]["display_name"]
                    .as_str()
                    .unwrap_or("Gemini 3.8 Flash (High)")
                    .to_string();

                return QuotaInfo {
                    context_tokens: tokens,
                    context_max: max_tokens,
                    used_pct,
                    five_hour_pct: five_h_rem,
                    five_hour_reset: format_duration(five_h_secs),
                    weekly_pct: weekly_rem,
                    weekly_reset: format_duration(weekly_secs),
                    active_model: model,
                };
            }
        }
    }

    // Fallback: read model from settings.json
    let mut info = QuotaInfo::default();
    if let Ok(content) = fs::read_to_string(settings_path()) {
        if let Ok(val) = serde_json::from_str::<Value>(&content) {
            if let Some(m) = val["model"].as_str() {
                info.active_model = m.to_string();
            }
        }
    }
    info
}

pub fn set_system_model(model_name: &str) -> Result<()> {
    let p = settings_path();
    let mut val: Value = if p.exists() {
        let content = fs::read_to_string(&p)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = val.as_object_mut() {
        obj.insert("model".to_string(), Value::String(model_name.to_string()));
    }

    let updated = serde_json::to_string_pretty(&val)?;
    fs::write(p, updated)?;
    Ok(())
}

pub fn load_chat_font_size() -> f32 {
    let p = settings_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if let Some(size) = val["chatFontSize"].as_f64() {
                    let s = size as f32;
                    if s >= 11.0 && s <= 24.0 {
                        return s;
                    }
                }
            }
        }
    }
    14.0 // 默认 14px 黄金标准
}

pub fn save_chat_font_size(size: f32) -> Result<()> {
    let p = settings_path();
    let mut val: Value = if p.exists() {
        let content = fs::read_to_string(&p)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = val.as_object_mut() {
        let rounded = (size * 10.0).round() / 10.0;
        obj.insert(
            "chatFontSize".to_string(),
            serde_json::Number::from_f64(rounded as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }

    let updated = serde_json::to_string_pretty(&val)?;
    fs::write(p, updated)?;
    Ok(())
}

pub fn load_theme_preference() -> bool {
    let p = settings_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if let Some(theme) = val["uiTheme"].as_str() {
                    return theme != "light";
                }
            }
        }
    }
    true // default to dark
}

pub fn save_theme_preference(is_dark: bool) -> Result<()> {
    let p = settings_path();
    let mut val: Value = if p.exists() {
        let content = fs::read_to_string(&p)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = val.as_object_mut() {
        obj.insert(
            "uiTheme".to_string(),
            Value::String(if is_dark { "dark".to_string() } else { "light".to_string() }),
        );
    }

    let updated = serde_json::to_string_pretty(&val)?;
    fs::write(p, updated)?;
    Ok(())
}

pub fn load_conversation_transcript(cid: &str) -> Vec<TranscriptEntry> {
    let path = base_dir()
        .join("brain")
        .join(cid)
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");

    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    let mut current_user_entry: Option<TranscriptEntry> = None;
    let mut latest_bot_content: Option<String> = None;
    let mut latest_bot_step: usize = 0;
    let mut latest_bot_time: String = String::new();

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let step_index = val["step_index"].as_u64().unwrap_or(0) as usize;
        let created_at = val["created_at"].as_str().unwrap_or("").to_string();
        let step_type = val["type"].as_str().unwrap_or("");
        let source = val["source"].as_str().unwrap_or("");

        if step_type == "USER_INPUT" || source == "USER_EXPLICIT" {
            // Flush previous turn's final bot answer if present
            if current_user_entry.is_some() {
                if let Some(bot_text) = latest_bot_content.take() {
                    entries.push(TranscriptEntry {
                        step_index: latest_bot_step,
                        entry_type: TranscriptType::Assistant,
                        content: bot_text,
                        timestamp: latest_bot_time.clone(),
                    });
                }
            }

            let mut raw_content = val["content"].as_str().unwrap_or("").to_string();
            // Clean up <USER_REQUEST> tags if present
            if let Some(start) = raw_content.find("<USER_REQUEST>") {
                let start_idx = start + "<USER_REQUEST>".len();
                if let Some(end) = raw_content[start_idx..].find("</USER_REQUEST>") {
                    raw_content = raw_content[start_idx..start_idx + end].trim().to_string();
                }
            }
            if !raw_content.is_empty() {
                let user_entry = TranscriptEntry {
                    step_index,
                    entry_type: TranscriptType::User,
                    content: raw_content,
                    timestamp: created_at,
                };
                entries.push(user_entry.clone());
                current_user_entry = Some(user_entry);
                latest_bot_content = None;
            }
        } else if step_type == "PLANNER_RESPONSE" {
            // Only capture final bot answer text; intermediate thinking and tool calls are omitted
            if let Some(content) = val["content"].as_str() {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    latest_bot_content = Some(trimmed.to_string());
                    latest_bot_step = step_index;
                    latest_bot_time = created_at;
                }
            }
        }
        // GENERIC, SYSTEM_MESSAGE, and intermediate steps are completely skipped
    }

    // Flush last turn's final bot answer
    if current_user_entry.is_some() {
        if let Some(bot_text) = latest_bot_content {
            entries.push(TranscriptEntry {
                step_index: latest_bot_step,
                entry_type: TranscriptType::Assistant,
                content: bot_text,
                timestamp: latest_bot_time,
            });
        }
    }

    entries
}

pub fn rename_session(cid: &str, new_title: &str) -> Result<()> {
    // 1. Update annotations/<cid>.pbtxt
    let annotations_dir = base_dir().join("annotations");
    let _ = fs::create_dir_all(&annotations_dir);
    let pbtxt = annotations_dir.join(format!("{}.pbtxt", cid));
    let _ = fs::write(pbtxt, format!("title:\"{}\"\n", new_title));

    // 2. Update SQLite DB
    let conn = Connection::open(db_path())?;
    conn.execute(
        "UPDATE conversation_summaries SET title = ?1 WHERE conversation_id = ?2",
        rusqlite::params![new_title, cid],
    )?;
    Ok(())
}

pub fn delete_session(cid: &str) -> Result<()> {
    let base = base_dir();
    let _ = fs::remove_dir_all(base.join("brain").join(cid));
    let _ = fs::remove_file(base.join("conversations").join(format!("{}.db", cid)));
    let _ = fs::remove_file(base.join("presence").join(format!("{}.lock", cid)));
    let _ = fs::remove_file(base.join("annotations").join(format!("{}.pbtxt", cid)));

    // Append to deleted_conversations.txt
    let del_file = deleted_file();
    let mut current = get_deleted_ids();
    current.insert(cid.to_string());
    let mut lines: Vec<_> = current.into_iter().collect();
    lines.sort();
    let _ = fs::write(del_file, lines.join("\n") + "\n");

    // Delete from SQLite DB
    let conn = Connection::open(db_path())?;
    conn.execute(
        "DELETE FROM conversation_summaries WHERE conversation_id = ?1",
        rusqlite::params![cid],
    )?;
    Ok(())
}

pub fn launch_terminal(cid: &str) -> Result<()> {
    let has_alacritty = Command::new("which")
        .arg("alacritty")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let has_konsole = Command::new("which")
        .arg("konsole")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_alacritty {
        Command::new("alacritty")
            .args(["-e", "agy", "--dangerously-skip-permissions", "--conversation", cid])
            .spawn()?;
    } else if has_konsole {
        Command::new("konsole")
            .args(["-e", "agy", "--dangerously-skip-permissions", "--conversation", cid])
            .spawn()?;
    } else {
        Command::new("xdg-terminal-exec")
            .args(["agy", "--dangerously-skip-permissions", "--conversation", cid])
            .spawn()?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn launch_terminal_with_prompt(cid: &str, prompt: &str) -> Result<()> {
    let has_alacritty = Command::new("which")
        .arg("alacritty")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let has_konsole = Command::new("which")
        .arg("konsole")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_alacritty {
        Command::new("alacritty")
            .args(["-e", "agy", "--dangerously-skip-permissions", "--conversation", cid, "-i", prompt])
            .spawn()?;
    } else if has_konsole {
        Command::new("konsole")
            .args(["-e", "agy", "--dangerously-skip-permissions", "--conversation", cid, "-i", prompt])
            .spawn()?;
    } else {
        Command::new("xdg-terminal-exec")
            .args(["agy", "--dangerously-skip-permissions", "--conversation", cid, "-i", prompt])
            .spawn()?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn launch_terminal_new_session(prompt: &str) -> Result<()> {
    let has_alacritty = Command::new("which")
        .arg("alacritty")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let has_konsole = Command::new("which")
        .arg("konsole")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_alacritty {
        Command::new("alacritty")
            .args(["-e", "agy", "--dangerously-skip-permissions", "-i", prompt])
            .spawn()?;
    } else if has_konsole {
        Command::new("konsole")
            .args(["-e", "agy", "--dangerously-skip-permissions", "-i", prompt])
            .spawn()?;
    } else {
        Command::new("xdg-terminal-exec")
            .args(["agy", "--dangerously-skip-permissions", "-i", prompt])
            .spawn()?;
    }
    Ok(())
}

pub fn run_agy_prompt(cid: Option<&str>, model: Option<&str>, prompt: &str) -> Result<String> {
    let mut cmd = Command::new("agy");
    cmd.arg("--dangerously-skip-permissions");
    if let Some(id) = cid {
        if !id.trim().is_empty() {
            cmd.args(["--conversation", id.trim()]);
        }
    }
    if let Some(m) = model {
        if !m.trim().is_empty() {
            cmd.args(["--model", m.trim()]);
        }
    }
    cmd.args(["-p", prompt]);

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stdout.is_empty() {
            Ok(stdout)
        } else {
            anyhow::bail!("Antigravity CLI 执行错误: {}", stderr)
        }
    }
}

