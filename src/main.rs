
const ANTIGRAVITY_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M19.94 20.59c1.09.82 2.73.27 1.23-1.23C16.67 15 17.62 3 12.03 3S7.39 15 2.89 19.36c-1.64 1.64.14 2.05 1.23 1.23c4.23-2.86 3.95-7.91 7.91-7.91s3.68 5.05 7.91 7.91"/></svg>"#;
mod session;

use gpui_kit::component::button::*;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::text::TextView;
use gpui_kit::component::Size as ComponentSize;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use session::{
    delete_session, launch_terminal, load_chat_font_size, load_conversation_transcript,
    load_quota_info, load_sessions, load_theme_preference, rename_session, run_agy_prompt,
    save_chat_font_size, save_theme_preference, set_system_model, ConversationItem, QuotaInfo,
    TranscriptEntry, TranscriptType,
};

pub fn detect_system_font() -> String {
    // 1. Try KDE 6
    if let Ok(output) = std::process::Command::new("kreadconfig6")
        .args(&["--group", "General", "--key", "font"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Some(font_name) = s.split(',').next() {
                let trimmed = font_name.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    // 2. Try KDE 5
    if let Ok(output) = std::process::Command::new("kreadconfig5")
        .args(&["--group", "General", "--key", "font"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Some(font_name) = s.split(',').next() {
                let trimmed = font_name.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    "霞鹜文楷等宽".to_string()
}

#[derive(Clone, Copy, Debug)]
pub struct ThemePalette {
    pub bg: u32,
    pub sidebar: u32,
    pub card: u32,
    pub card_active: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub claude: u32,
    pub green: u32,
    pub yellow: u32,
    pub cyan: u32,
}

impl ThemePalette {
    pub fn claude_alpha(&self, alpha: u8) -> u32 {
        (self.claude << 8) | (alpha as u32)
    }
    pub fn green_alpha(&self, alpha: u8) -> u32 {
        (self.green << 8) | (alpha as u32)
    }
    pub fn yellow_alpha(&self, alpha: u8) -> u32 {
        (self.yellow << 8) | (alpha as u32)
    }
    pub fn cyan_alpha(&self, alpha: u8) -> u32 {
        (self.cyan << 8) | (alpha as u32)
    }
    pub fn muted_alpha(&self, alpha: u8) -> u32 {
        (self.muted << 8) | (alpha as u32)
    }
}

fn format_time(ts: &str) -> String {
    if let Some(t_idx) = ts.find('T') {
        let after_t = &ts[t_idx + 1..];
        if after_t.len() >= 5 {
            return after_t[..5].to_string();
        }
    } else if let Some(space_idx) = ts.find(' ') {
        let after_space = &ts[space_idx + 1..];
        if after_space.len() >= 5 {
            return after_space[..5].to_string();
        }
    }
    if ts.len() >= 5 {
        ts[..5].to_string()
    } else {
        ts.to_string()
    }
}

// 🌙 Claude Code Dark Theme (Warm Charcoal)
pub const PALETTE_DARK: ThemePalette = ThemePalette {
    bg: 0x1F1E1D,          // Deep warm charcoal canvas
    sidebar: 0x191817,     // Slightly darker sidebar
    card: 0x272623,        // Elevated card background
    card_active: 0x32302C, // Selected session item
    border: 0x3D3935,      // Clean muted border
    text: 0xF0EBE1,        // Warm off-white text
    muted: 0x9E978E,       // Muted gray-brown text
    claude: 0xD97757,      // Iconic Claude terracotta orange
    green: 0x78AF6E,       // Warm terminal success green
    yellow: 0xE6B450,      // Warm terminal yellow / warning
    cyan: 0x85B5A3,        // Muted tool / cyan accent
};

// ☀️ Claude Code Light Theme (Warm Paper / Cream)
pub const PALETTE_LIGHT: ThemePalette = ThemePalette {
    bg: 0xFAF8F5,          // Warm cream paper canvas
    sidebar: 0xF2ECE4,     // Warm parchment sidebar
    card: 0xFFFFFF,        // Pure white card
    card_active: 0xE8E1D5, // Warm beige selected session
    border: 0xDCD5CB,      // Warm light border
    text: 0x242220,        // High-contrast warm charcoal text
    muted: 0x736D66,       // Readable muted text
    claude: 0xC45532,      // Rich terracotta orange
    green: 0x2D7A38,       // Rich forest green
    yellow: 0xA66E0A,      // Warm amber yellow
    cyan: 0x1B7470,        // Deep teal
};

pub struct ModelOption {
    pub name: &'static str,
    pub desc: &'static str,
    pub tag: &'static str,
}

pub const AVAILABLE_MODELS: &[ModelOption] = &[
    ModelOption {
        name: "Gemini 3.8 Flash (High)",
        desc: "极速响应 · 深度长链路思考推理 (默认推荐)",
        tag: "推荐",
    },
    ModelOption {
        name: "Gemini 3.8 Flash (Medium)",
        desc: "平衡推理 · 适合日常开发与常规编码",
        tag: "均衡",
    },
    ModelOption {
        name: "Gemini 3.8 Flash (Low)",
        desc: "极低延迟 · 快速问答与轻量任务",
        tag: "极速",
    },
    ModelOption {
        name: "Claude Sonnet 4.6 (Thinking)",
        desc: "顶尖高智力模型 · 架构设计与复杂重构",
        tag: "Thinking",
    },
    ModelOption {
        name: "Claude Opus 4.6 (Thinking)",
        desc: "最强大脑 · 深度全链路推理",
        tag: "Pro",
    },
    ModelOption {
        name: "GPT-OSS 120B (Medium)",
        desc: "开源顶级权重推理 · 高阶泛化能力",
        tag: "开源",
    },
];

pub struct AntiCliApp {
    sessions: Vec<ConversationItem>,
    filtered_sessions: Vec<ConversationItem>,
    selected_cid: Option<String>,
    transcript: Vec<TranscriptEntry>,

    search_query: String,
    search_input: Entity<InputState>,
    prompt_input: Entity<InputState>,

    // Quota and Model Info
    quota: QuotaInfo,
    selected_model: String,
    show_model_menu: bool,

    // Rename state
    renaming_item: Option<ConversationItem>,
    rename_input: Entity<InputState>,

    // Delete confirmation state
    deleting_item: Option<ConversationItem>,

    // Settings Modal State
    show_settings: bool,
    settings_tab: usize, // 0: 外观与主题, 1: 模型与智能体, 2: 终端与集成, 3: 关于
    username: String,
    auto_skip_permissions: bool,

    // Status message & Persistent theme
    status_msg: Option<String>,
    is_dark: bool,

    // In-app Direct Generation State
    is_generating: bool,
    chat_font_size: f32,
    system_font: String,
}

impl AntiCliApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let system_font = detect_system_font();
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("搜索会话 (按关键词或 UUID)...")
        });

        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入指令直接对话，或使用斜杠命令 (例如 /compact, /resume)...")
        });

        let rename_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("输入新的会话标题...")
        });

        let username = std::env::var("USER").unwrap_or_else(|_| "rookstein".to_string());
        let quota = load_quota_info();
        let selected_model = quota.active_model.clone();
        let is_dark = load_theme_preference();

        let mut app = Self {
            sessions: Vec::new(),
            filtered_sessions: Vec::new(),
            selected_cid: None,
            transcript: Vec::new(),
            search_query: String::new(),
            search_input: search_input.clone(),
            prompt_input: prompt_input.clone(),
            quota,
            selected_model,
            show_model_menu: false,
            renaming_item: None,
            rename_input,
            deleting_item: None,
            show_settings: false,
            settings_tab: 0,
            username,
            auto_skip_permissions: true,
            status_msg: Some("Claude Code 界面就绪".to_string()),
            is_dark,
            is_generating: false,
            chat_font_size: load_chat_font_size(),
            system_font,
        };

        // Load sessions
        app.reload_sessions();

        // Subscribe to search input changes
        cx.subscribe(&search_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.search_query = this.search_input.read(cx).value().to_string();
                this.filter_sessions();
                cx.notify();
            }
        })
        .detach();

        // Subscribe to prompt input enter key (Direct in-app execution without popping up external terminal)
        cx.subscribe_in(&prompt_input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { shift: false, .. } = event {
                this.submit_prompt(window, cx);
            }
        })
        .detach();

        app
    }

    pub fn reload_sessions(&mut self) {
        match load_sessions() {
            Ok(list) => {
                self.sessions = list;
                self.filter_sessions();

                // Select the first session by default if none selected
                if self.selected_cid.is_none() && !self.sessions.is_empty() {
                    let first_id = self.sessions[0].id.clone();
                    self.select_session(first_id);
                }
            }
            Err(e) => {
                self.status_msg = Some(format!("加载会话失败: {}", e));
            }
        }
    }

    pub fn select_session(&mut self, cid: String) {
        self.selected_cid = Some(cid.clone());
        self.transcript = load_conversation_transcript(&cid);
    }

    pub fn filter_sessions(&mut self) {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
        } else {
            self.filtered_sessions = self
                .sessions
                .iter()
                .filter(|s| {
                    s.title.to_lowercase().contains(&query)
                        || s.id.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
    }

    pub fn switch_model(&mut self, model_name: String, cx: &mut Context<Self>) {
        let _ = set_system_model(&model_name);
        self.selected_model = model_name.clone();
        self.quota.active_model = model_name.clone();
        self.show_model_menu = false;
        self.status_msg = Some(format!("✓ 已切换当前使用的模型: {}", model_name));
        cx.notify();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.is_dark = !self.is_dark;
        let _ = save_theme_preference(self.is_dark);
        let mode = if self.is_dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        let font_fam = self.system_font.clone();
        Theme::change(mode, None, cx);
        Theme::update(cx, |theme| {
            theme.font_family = font_fam.clone().into();
            theme.mono_font_family = font_fam.into();
        });
        self.status_msg = Some(format!(
            "✓ 主题已保存: {}",
            if self.is_dark { "Claude Dark (暗夜黑)" } else { "Claude Light (白昼暖白)" }
        ));
        cx.notify();
    }

    pub fn set_chat_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        let clamped = size.clamp(11.0, 24.0);
        self.chat_font_size = (clamped * 10.0).round() / 10.0;
        let _ = save_chat_font_size(self.chat_font_size);
        self.status_msg = Some(format!("✓ 聊天字号已保存: {:.1}px", self.chat_font_size));
        cx.notify();
    }

    pub fn zoom_in_font(&mut self, cx: &mut Context<Self>) {
        self.set_chat_font_size(self.chat_font_size + 1.0, cx);
    }

    pub fn zoom_out_font(&mut self, cx: &mut Context<Self>) {
        self.set_chat_font_size(self.chat_font_size - 1.0, cx);
    }

    pub fn reset_zoom_font(&mut self, cx: &mut Context<Self>) {
        self.set_chat_font_size(14.0, cx);
    }

    pub fn start_rename(
        &mut self,
        item: ConversationItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = item.title.clone();
        self.renaming_item = Some(item);
        self.rename_input.update(cx, |input, cx| {
            input.set_value(&title, window, cx);
        });
        cx.notify();
    }

    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_item = None;
        cx.notify();
    }

    pub fn save_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.renaming_item.take() {
            let new_title = self.rename_input.read(cx).value().to_string();
            let new_title = new_title.trim();
            if !new_title.is_empty() {
                match rename_session(&target.id, new_title) {
                    Ok(_) => {
                        self.status_msg = Some(format!("✓ 已重命名为: 【{}】", new_title));
                        self.reload_sessions();
                    }
                    Err(e) => {
                        self.status_msg = Some(format!("重命名失败: {}", e));
                    }
                }
            }
        }
        cx.notify();
    }

    pub fn start_delete(&mut self, item: ConversationItem, cx: &mut Context<Self>) {
        self.deleting_item = Some(item);
        cx.notify();
    }

    pub fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.deleting_item = None;
        cx.notify();
    }

    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.deleting_item.take() {
            match delete_session(&target.id) {
                Ok(_) => {
                    self.status_msg = Some(format!("✓ 已删除会话: 【{}】", target.title));
                    if self.selected_cid.as_deref() == Some(&target.id) {
                        self.selected_cid = None;
                        self.transcript.clear();
                    }
                    self.reload_sessions();
                }
                Err(e) => {
                    self.status_msg = Some(format!("删除会话失败: {}", e));
                }
            }
        }
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn resume_in_terminal(&mut self, cid: &str, cx: &mut Context<Self>) {
        match launch_terminal(cid) {
            Ok(_) => {
                self.status_msg = Some("✓ Alacritty 终端已唤起 Claude/Antigravity 会话".to_string());
            }
            Err(e) => {
                self.status_msg = Some(format!("唤起终端失败: {}", e));
            }
        }
        cx.notify();
    }

    pub fn submit_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_generating {
            return;
        }
        let val = self.prompt_input.read(cx).value().to_string();
        let prompt = val.trim();
        if prompt.is_empty() {
            return;
        }
        let prompt_str = prompt.to_string();
        self.prompt_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.transcript.push(TranscriptEntry {
            step_index: self.transcript.len(),
            entry_type: TranscriptType::User,
            content: prompt_str.clone(),
            timestamp: now,
        });

        self.is_generating = true;
        self.status_msg = Some(format!("✦ 正在思考回复中..."));
        cx.notify();

        let cid_opt = self.selected_cid.clone();
        let model_opt = Some(self.selected_model.clone());
        let prompt_clone = prompt_str.clone();

        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                run_agy_prompt(cid_opt.as_deref(), model_opt.as_deref(), &prompt_clone)
            }).await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |app, cx| {
                    app.is_generating = false;
                    match result {
                        Ok(reply) => {
                            if let Ok(sessions) = session::load_sessions() {
                                app.sessions = sessions;
                                app.filter_sessions();
                            }
                            if app.selected_cid.is_none() {
                                if let Some(newest) = app.sessions.first() {
                                    app.selected_cid = Some(newest.id.clone());
                                }
                            }

                            if let Some(ref cid) = app.selected_cid {
                                let disk = session::load_conversation_transcript(cid);
                                if !disk.is_empty() {
                                    app.transcript = disk;
                                } else {
                                    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                                    app.transcript.push(TranscriptEntry {
                                        step_index: app.transcript.len(),
                                        entry_type: TranscriptType::Assistant,
                                        content: reply,
                                        timestamp: now_str,
                                    });
                                }
                            } else {
                                let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                                app.transcript.push(TranscriptEntry {
                                    step_index: app.transcript.len(),
                                    entry_type: TranscriptType::Assistant,
                                    content: reply,
                                    timestamp: now_str,
                                });
                            }

                            app.quota = session::load_quota_info();
                            app.status_msg = Some("✓ 回复生成完成".to_string());
                        }
                        Err(e) => {
                            app.status_msg = Some(format!("生成失败: {}", e));
                            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                            app.transcript.push(TranscriptEntry {
                                step_index: app.transcript.len(),
                                entry_type: TranscriptType::Assistant,
                                content: format!("⚠️ **生成失败**: {}", e),
                                timestamp: now_str,
                            });
                        }
                    }
                    cx.notify();
                });
            }
        }).detach();
    }
}

impl Render for AntiCliApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_cid = self.selected_cid.clone();
        let selected_session = self
            .sessions
            .iter()
            .find(|s| Some(&s.id) == active_cid.as_ref())
            .cloned();

        let active_title = selected_session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_else(|| "无活跃会话".to_string());

        let total_sessions = self.sessions.len();
        let filtered_sessions = self.filtered_sessions.len();

        let show_settings = self.show_settings;
        let current_tab = self.settings_tab;
        let is_dark_mode = self.is_dark;
        let cur_model = self.selected_model.clone();
        let skip_perm = self.auto_skip_permissions;
        let show_model_menu = self.show_model_menu;

        // Dynamic theme palette based on user preference
        let p = if is_dark_mode { PALETTE_DARK } else { PALETTE_LIGHT };

        // Quota calculations
        let _context_k = self.quota.context_tokens as f64 / 1000.0;
        let used_pct = self.quota.used_pct;
        let five_h_pct = self.quota.five_hour_pct;
        let five_h_reset = self.quota.five_hour_reset.clone();
        let weekly_pct = self.quota.weekly_pct;
        let weekly_reset = self.quota.weekly_reset.clone();
        let cur_font_size = self.chat_font_size;

        // Outer Container: Standard rectangular desktop window & system font
        div()
            .id("claude_code_app")
            .font_family(self.system_font.clone())
            .size_full()
            .border_1()
            .border_color(rgb(p.border))
            .bg(rgb(p.bg))
            .text_color(rgb(p.text))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control {
                    match event.keystroke.key.as_str() {
                        "=" | "+" => this.zoom_in_font(cx),
                        "-" => this.zoom_out_font(cx),
                        "0" => this.reset_zoom_font(cx),
                        _ => {}
                    }
                }
            }))
            .flex()
            .flex_col()
            // ─────────────────────────────────────────────────────────────
            // 1. Antigravity Top Header Bar (Draggable)
            // ─────────────────────────────────────────────────────────────
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(46.0))
                    .w_full()
                    .bg(rgb(p.sidebar))
                    .border_b_1()
                    .border_color(rgb(p.border))
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    // Left Brand: Antigravity Icon & Name
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2p5()
                            .child(
                                div()
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded_md()
                                    .bg(rgb(p.claude))
                                    .text_color(rgb(0xFFFFFF))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        svg()
                                            .data(ANTIGRAVITY_SVG)
                                            .w(px(16.0))
                                            .h(px(16.0)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(p.text))
                                    .child("Antigravity"),
                            ),
                    )
                    // Center Draggable Window Area
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .cursor(CursorStyle::OpenHand)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            }),
                    )
                    // Right Window Controls: Zoom (Maximize) & Close
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            // Window Maximize
                            .child(
                                Button::new("btn_zoom")
                                    .ghost()
                                    .with_size(ComponentSize::Small)
                                    .label("🗖")
                                    .on_click(|_, window, _| {
                                        window.zoom_window();
                                    }),
                            )
                            // Window Close (Instant Teardown)
                            .child(
                                Button::new("btn_close")
                                    .ghost()
                                    .with_size(ComponentSize::Small)
                                    .label("✕")
                                    .on_click(|_, window, cx| {
                                        window.remove_window();
                                        cx.quit();
                                        std::process::exit(0);
                                    }),
                            ),
                    ),
            )
            // ─────────────────────────────────────────────────────────────
            // 2. Main Workstation Body: Split Left (3-Tier) + Right (Timeline)
            // ─────────────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    // ─────────────────────────────────────────────────────────
                    // Left Panel: 3 Vertical Regions (Top Reserved, Mid Sessions, Bottom User+Settings)
                    // ─────────────────────────────────────────────────────────
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(px(320.0))
                            .h_full()
                            .bg(rgb(p.sidebar))
                            .border_r_1()
                            .border_color(rgb(p.border))
                            .flex()
                            .flex_col()
                            // ─────────────────────────────────────────────────
                            // 【左侧上方】：预留区域 (固定高度 115px)
                            // ─────────────────────────────────────────────────
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .h(px(115.0))
                                    .p_3()
                                    .border_b_1()
                                    .border_color(rgb(p.border))
                                    .child(
                                        div()
                                            .size_full()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(rgba(p.claude_alpha(0x44)))
                                            .bg(rgba(p.claude_alpha(0x0C)))
                                            .p_3()
                                            .flex()
                                            .flex_col()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(rgb(p.claude))
                                                            .child("✦ 预留功能区 (Reserved Area)"),
                                                    )
                                                    .child(
                                                        div()
                                                            .px_1p5()
                                                            .py(px(0.5))
                                                            .rounded_sm()
                                                            .bg(rgba(p.claude_alpha(0x22)))
                                                            .text_xs()
                                                            .text_color(rgb(p.claude))
                                                            .child("Slot #1"),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(p.muted))
                                                    .child("左侧顶部空间已锁定，支持未来直接拓展快捷工具箱、智能体技能卡或全局工作台。"),
                                            ),
                                    ),
                            )
                            // ─────────────────────────────────────────────────
                            // 【左侧中间】：会话管理 (自适应撑满 flex-1 + min_h_0)
                            // ─────────────────────────────────────────────────
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .flex()
                                    .flex_col()
                                    // Search Bar & Stats Header
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .p_3()
                                            .border_b_1()
                                            .border_color(rgb(p.border))
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(p.muted))
                                                    .child("CONVERSATIONS (会话管理)")
                                                    .child(format!("{}/{} 会话", filtered_sessions, total_sessions)),
                                            )
                                            .child(Input::new(&self.search_input).with_size(ComponentSize::Small)),
                                    )
                                    // Sessions List (Scrollable)
                                    .child(
                                        div()
                                            .id("sessions_scroll_list")
                                            .flex_1()
                                            .min_h_0()
                                            .overflow_y_scroll()
                                            .p_2()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .children(self.filtered_sessions.iter().enumerate().map(|(idx, item)| {
                                                let is_selected = Some(&item.id) == active_cid.as_ref();
                                                let cid = item.id.clone();
                                                let item_clone = item.clone();

                                                div()
                                                    .id(format!("session_item_{}", idx))
                                                    .p_2p5()
                                                    .rounded_md()
                                                    .cursor(CursorStyle::PointingHand)
                                                    .bg(if is_selected {
                                                        rgb(p.card_active)
                                                    } else {
                                                        rgba(0x00000000)
                                                    })
                                                    .border_l_2()
                                                    .border_color(if is_selected {
                                                        rgb(p.claude)
                                                    } else {
                                                        rgba(0x00000000)
                                                    })
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.select_session(cid.clone());
                                                        cx.notify();
                                                    }))
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1p5()
                                                    // Title + Step Count Badge
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_weight(FontWeight::BOLD)
                                                                    .text_color(if is_selected {
                                                                        rgb(p.claude)
                                                                    } else {
                                                                        rgb(p.text)
                                                                    })
                                                                    .child(item.title.clone()),
                                                            )
                                                            .child(
                                                                div()
                                                                    .px_1p5()
                                                                    .py(px(0.5))
                                                                    .rounded_sm()
                                                                    .bg(rgba(p.claude_alpha(0x18)))
                                                                    .text_xs()
                                                                    .text_color(rgb(p.claude))
                                                                    .child(format!("{}步", item.step_count)),
                                                            ),
                                                    )
                                                    // Timestamp + Action buttons (Rename & Delete)
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .text_xs()
                                                            .text_color(rgb(p.muted))
                                                            .child(item.last_modified.clone())
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_1()
                                                                    .child(
                                                                        Button::new(format!("btn_rn_{}", idx))
                                                                            .ghost()
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("✏️")
                                                                            .on_click(cx.listener({
                                                                                let item_for_rn = item_clone.clone();
                                                                                move |this, _, window, cx| {
                                                                                    this.start_rename(item_for_rn.clone(), window, cx);
                                                                                }
                                                                            })),
                                                                    )
                                                                    .child(
                                                                        Button::new(format!("btn_dl_{}", idx))
                                                                            .ghost()
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("🗑️")
                                                                            .on_click(cx.listener({
                                                                                let item_for_dl = item_clone;
                                                                                move |this, _, _, cx| {
                                                                                    this.start_delete(item_for_dl.clone(), cx);
                                                                                }
                                                                            })),
                                                                    ),
                                                            ),
                                                    )
                                                    .into_any_element()
                                            })),
                                    ),
                            )
                            // ─────────────────────────────────────────────────
                            // 【左侧下方】：当前用户名称 + 设置按钮 (固定高度 64px)
                            // ─────────────────────────────────────────────────
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .h(px(64.0))
                                    .px_3()
                                    .bg(rgb(p.card))
                                    .border_t_1()
                                    .border_color(rgb(p.border))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    // User Avatar & Name
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2p5()
                                            .child(
                                                div()
                                                    .w(px(32.0))
                                                    .h(px(32.0))
                                                    .rounded_full()
                                                    .bg(rgb(p.claude))
                                                    .text_color(rgb(0xFFFFFF))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_sm()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child("R"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_0p5()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(rgb(p.text))
                                                            .child(self.username.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_1p5()
                                                            .text_xs()
                                                            .text_color(rgb(p.green))
                                                            .child("● 在线 · CachyOS"),
                                                    ),
                                            ),
                                    )
                                    // Settings Button
                                    .child(
                                        Button::new("btn_open_settings")
                                            .secondary()
                                            .with_size(ComponentSize::Small)
                                            .label("⚙️ 设置")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_settings = true;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    // ─────────────────────────────────────────────────────────
                    // Right Panel: Claude Code Timeline & Bottom Fixed Chat + Quota Bar
                    // ─────────────────────────────────────────────────────────
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .bg(rgb(p.bg))
                            .flex()
                            .flex_col()
                            // Top Active Session Toolbar (Fixed height 52px)
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .h(px(52.0))
                                    .px_6()
                                    .bg(rgb(p.card))
                                    .border_b_1()
                                    .border_color(rgb(p.border))
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2p5()
                                            .child(
                                                div()
                                                    .text_base()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(p.text))
                                                    .child(active_title),
                                            )
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py(px(1.0))
                                                    .rounded_full()
                                                    .bg(if active_cid.is_some() {
                                                        rgba(p.green_alpha(0x22))
                                                    } else {
                                                        rgba(p.muted_alpha(0x22))
                                                    })
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(if active_cid.is_some() {
                                                        rgb(p.green)
                                                    } else {
                                                        rgb(p.muted)
                                                    })
                                                    .child(if active_cid.is_some() { "● Active" } else { "○ Inactive" }),
                                            ),
                                    ),
                            )
                            // Central Transcript Timeline (Scrollable flex-1 + min_h_0)
                            .child(
                                div()
                                    .id("transcript_timeline")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .px_6()
                                    .py_4()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .children(if self.transcript.is_empty() && !self.is_generating {
                                        vec![
                                            div()
                                                .flex_1()
                                                .h(px(280.0))
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .justify_center()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .w(px(48.0))
                                                        .h(px(48.0))
                                                        .rounded_xl()
                                                        .bg(rgba(p.claude_alpha(0x22)))
                                                        .text_color(rgb(p.claude))
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_xl()
                                                        .child("✦"),
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(rgb(p.text))
                                                        .child("Claude Code 交互就绪"),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(p.muted))
                                                        .child("在下方固定输入框输入指令直接对话，或选择左侧会话回放"),
                                                )
                                                .into_any_element()
                                        ]
                                    } else {
                                        let mut items: Vec<AnyElement> = self.transcript.iter().enumerate().map(|(idx, entry)| {
                                            match &entry.entry_type {
                                                TranscriptType::User => {
                                                    let time_str = format_time(&entry.timestamp);
                                                    div()
                                                        .id(format!("wrap_user_{}", idx))
                                                        .w_full()
                                                        .flex()
                                                        .justify_end()
                                                        .child(
                                                            div()
                                                                .max_w(relative(0.48))
                                                                .min_w_0()
                                                                .flex()
                                                                .flex_col()
                                                                .items_end()
                                                                .gap_1()
                                                                .child(
                                                                    div()
                                                                        .id(format!("bubble_user_{}", idx))
                                                                        .text_size(px(cur_font_size))
                                                                        .px_4()
                                                                        .py_3()
                                                                        .rounded_2xl()
                                                                        .rounded_br_sm()
                                                                        .bg(if is_dark_mode {
                                                                            rgba(p.claude_alpha(0x2C))
                                                                        } else {
                                                                            rgba(p.claude_alpha(0x18))
                                                                        })
                                                                        .border_1()
                                                                        .border_color(rgba(p.claude_alpha(0x55)))
                                                                        .shadow_sm()
                                                                        .flex()
                                                                        .flex_col()
                                                                        .gap_1p5()
                                                                        .child(
                                                                            div()
                                                                                .flex()
                                                                                .items_center()
                                                                                .gap_2()
                                                                                .pb_1p5()
                                                                                .border_b_1()
                                                                                .border_color(rgba(p.muted_alpha(0x1F)))
                                                                                .child(
                                                                                    div()
                                                                                        .text_xs()
                                                                                        .font_weight(FontWeight::BOLD)
                                                                                        .text_color(rgb(p.claude))
                                                                                        .child(format!("👤 {}", self.username)),
                                                                                )
                                                                                .child(
                                                                                    div()
                                                                                        .text_xs()
                                                                                        .text_color(rgb(p.muted))
                                                                                        .child(format!("· {}", time_str)),
                                                                                ),
                                                                        )
                                                                        .child(
                                                                            TextView::markdown(
                                                                                format!("user_md_{}", idx),
                                                                                entry.content.clone(),
                                                                            )
                                                                            .selectable(true),
                                                                        ),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .justify_end()
                                                                        .pr_1()
                                                                        .child({
                                                                            let copy_text = entry.content.clone();
                                                                            Button::new(format!("btn_copy_user_{}", idx))
                                                                                .ghost()
                                                                                .with_size(ComponentSize::Small)
                                                                                .icon(IconName::Copy)
                                                                                .tooltip("复制内容")
                                                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                                                    cx.write_to_clipboard(ClipboardItem::new_string(copy_text.clone()));
                                                                                    this.status_msg = Some("✓ 内容已复制到剪贴板".to_string());
                                                                                    cx.notify();
                                                                                }))
                                                                        }),
                                                                ),
                                                        )
                                                        .into_any_element()
                                                }
                                                TranscriptType::Thinking => {
                                                    let time_str = format_time(&entry.timestamp);
                                                    div()
                                                        .id(format!("wrap_think_{}", idx))
                                                        .w_full()
                                                        .flex()
                                                        .justify_start()
                                                        .items_start()
                                                        .child(
                                                            div()
                                                                .max_w(relative(0.48))
                                                                .min_w_0()
                                                                .flex()
                                                                .items_start()
                                                                .gap_3()
                                                                .child(
                                                                    div()
                                                                        .flex_shrink_0()
                                                                        .w(px(34.0))
                                                                        .h(px(34.0))
                                                                        .rounded_full()
                                                                        .bg(rgba(p.yellow_alpha(0x22)))
                                                                        .text_color(rgb(p.yellow))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .text_sm()
                                                                        .child("🧠"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(format!("bubble_think_{}", idx))
                                                                        .flex_1()
                                                                        .min_w_0()
                                                                        .px_4()
                                                                        .py_3()
                                                                        .rounded_xl()
                                                                        .rounded_tl_sm()
                                                                        .bg(rgba(p.yellow_alpha(0x0E)))
                                                                        .border_1()
                                                                        .border_color(rgba(p.yellow_alpha(0x33)))
                                                                        .border_l_4()
                                                                        .border_color(rgb(p.yellow))
                                                                        .flex()
                                                                        .flex_col()
                                                                        .gap_1p5()
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_between()
                                                                        .gap_3()
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .font_weight(FontWeight::BOLD)
                                                                                .text_color(rgb(p.yellow))
                                                                                .child("🧠 思考推理 (Thinking Process)"),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .text_color(rgb(p.muted))
                                                                                .child(time_str),
                                                                        ),
                                                                )
                                                                .child(
                                                                    TextView::markdown(
                                                                        format!("think_md_{}", idx),
                                                                        entry.content.clone(),
                                                                    )
                                                                    .selectable(true),
                                                                ),
                                                        ),
                                                        )
                                                        .into_any_element()
                                                }
                                                TranscriptType::ToolCall { name, args } => {
                                                    div()
                                                        .id(format!("wrap_tool_{}", idx))
                                                        .w_full()
                                                        .flex()
                                                        .justify_start()
                                                        .items_start()
                                                        .child(
                                                            div()
                                                                .max_w(relative(0.48))
                                                                .min_w_0()
                                                                .flex()
                                                                .items_start()
                                                                .gap_3()
                                                                .child(
                                                                    div()
                                                                        .flex_shrink_0()
                                                                        .w(px(34.0))
                                                                        .h(px(34.0))
                                                                        .rounded_full()
                                                                        .bg(rgba(p.green_alpha(0x22)))
                                                                        .text_color(rgb(p.green))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .text_sm()
                                                                        .child("⚡"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(format!("bubble_tool_{}", idx))
                                                                        .flex_1()
                                                                        .min_w_0()
                                                                .p_3()
                                                                .rounded_xl()
                                                                .rounded_tl_sm()
                                                                .bg(if is_dark_mode { rgb(0x181716) } else { rgb(0xF2EDE6) })
                                                                .border_1()
                                                                .border_color(rgba(p.green_alpha(0x44)))
                                                                .border_l_4()
                                                                .border_color(rgb(p.green))
                                                                .flex()
                                                                .flex_col()
                                                                .gap_1p5()
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_between()
                                                                        .child(
                                                                            div()
                                                                                .flex()
                                                                                .items_center()
                                                                                .gap_2()
                                                                                .child(
                                                                                    div()
                                                                                        .text_xs()
                                                                                        .font_weight(FontWeight::BOLD)
                                                                                        .text_color(rgb(p.green))
                                                                                        .child(format!("⚡ 工具执行: {}", name)),
                                                                                ),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .px_1p5()
                                                                                .py(px(0.5))
                                                                                .rounded_sm()
                                                                                .bg(rgba(p.green_alpha(0x33)))
                                                                                .text_xs()
                                                                                .font_weight(FontWeight::BOLD)
                                                                                .text_color(rgb(p.green))
                                                                                .child("✓ DONE"),
                                                                        ),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .p_2()
                                                                        .rounded_md()
                                                                        .bg(if is_dark_mode { rgb(0x121110) } else { rgb(0xEAE4DB) })
                                                                        .text_xs()
                                                                        .text_color(rgb(p.muted))
                                                                        .child(args.clone()),
                                                                ),
                                                        ),
                                                        )
                                                        .into_any_element()
                                                }
                                                TranscriptType::ToolResult { output } => {
                                                    div()
                                                        .id(format!("wrap_res_{}", idx))
                                                        .w_full()
                                                        .flex()
                                                        .justify_start()
                                                        .pl(px(46.0))
                                                        .child(
                                                            div()
                                                                .id(format!("bubble_res_{}", idx))
                                                                .max_w(relative(0.48))
                                                                .min_w_0()
                                                                .p_2p5()
                                                                .rounded_lg()
                                                                .bg(if is_dark_mode { rgb(0x141312) } else { rgb(0xEAE3D8) })
                                                                .border_1()
                                                                .border_color(rgb(p.border))
                                                                .child(
                                                                    TextView::markdown(
                                                                        format!("res_md_{}", idx),
                                                                        format!("```text\n{}\n```", output),
                                                                    )
                                                                    .selectable(true),
                                                                ),
                                                        )
                                                        .into_any_element()
                                                }
                                                TranscriptType::Assistant => {
                                                    let time_str = format_time(&entry.timestamp);
                                                    div()
                                                        .id(format!("wrap_asst_{}", idx))
                                                        .w_full()
                                                        .flex()
                                                        .justify_start()
                                                        .items_start()
                                                        .child(
                                                            div()
                                                                .max_w(relative(0.48))
                                                                .min_w_0()
                                                                .flex()
                                                                .items_start()
                                                                .gap_3()
                                                                .child(
                                                                    div()
                                                                        .flex_shrink_0()
                                                                        .w(px(34.0))
                                                                        .h(px(34.0))
                                                                        .rounded_full()
                                                                        .bg(rgb(p.claude))
                                                                        .text_color(rgb(0xFFFFFF))
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .text_sm()
                                                                        .font_weight(FontWeight::BOLD)
                                                                        .child("✦"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .min_w_0()
                                                                        .flex()
                                                                        .flex_col()
                                                                        .items_end()
                                                                        .gap_1()
                                                                        .child(
                                                                            div()
                                                                                .id(format!("bubble_asst_{}", idx))
                                                                                .w_full()
                                                                                .text_size(px(cur_font_size))
                                                                                .px_4()
                                                                                .py_3()
                                                                                .rounded_2xl()
                                                                                .rounded_tl_sm()
                                                                                .bg(rgb(p.card))
                                                                                .border_1()
                                                                                .border_color(rgb(p.border))
                                                                                .shadow_sm()
                                                                                .flex()
                                                                                .flex_col()
                                                                                .gap_2()
                                                                                .child(
                                                                                    div()
                                                                                        .flex()
                                                                                        .items_center()
                                                                                        .gap_2()
                                                                                        .pb_1p5()
                                                                                        .border_b_1()
                                                                                        .border_color(rgba(p.muted_alpha(0x1F)))
                                                                                        .child(
                                                                                            div()
                                                                                                .text_xs()
                                                                                                .font_weight(FontWeight::BOLD)
                                                                                                .text_color(rgb(p.claude))
                                                                                                .child("✦ Claude Code"),
                                                                                        )
                                                                                        .child(
                                                                                            div()
                                                                                                .px_1p5()
                                                                                                .py(px(0.5))
                                                                                                .rounded_sm()
                                                                                                .bg(rgba(p.claude_alpha(0x22)))
                                                                                                .text_xs()
                                                                                                .font_weight(FontWeight::BOLD)
                                                                                                .text_color(rgb(p.claude))
                                                                                                .child("BOT"),
                                                                                        )
                                                                                        .child(
                                                                                            div()
                                                                                                .text_xs()
                                                                                                .text_color(rgb(p.muted))
                                                                                                .child(format!("· {}", time_str)),
                                                                                        ),
                                                                                )
                                                                                .child(
                                                                                    TextView::markdown(
                                                                                        format!("asst_md_{}", idx),
                                                                                        entry.content.clone(),
                                                                                    )
                                                                                    .selectable(true),
                                                                                ),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .flex()
                                                                                .justify_end()
                                                                                .pr_1()
                                                                                .child({
                                                                                    let copy_text = entry.content.clone();
                                                                                    Button::new(format!("btn_copy_asst_{}", idx))
                                                                                        .ghost()
                                                                                        .with_size(ComponentSize::Small)
                                                                                        .icon(IconName::Copy)
                                                                                        .tooltip("复制内容")
                                                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                                                            cx.write_to_clipboard(ClipboardItem::new_string(copy_text.clone()));
                                                                                            this.status_msg = Some("✓ 回复已复制到剪贴板".to_string());
                                                                                            cx.notify();
                                                                                        }))
                                                                                }),
                                                                        ),
                                                                ),
                                                        )
                                                        .into_any_element()
                                                }
                                            }
                                        }).collect();

                                        if self.is_generating {
                                            items.push(
                                                div()
                                                    .id("wrap_typing_indicator")
                                                    .w_full()
                                                    .flex()
                                                    .justify_start()
                                                    .items_start()
                                                    .child(
                                                        div()
                                                            .max_w(relative(0.48))
                                                            .min_w_0()
                                                            .flex()
                                                            .items_start()
                                                            .gap_3()
                                                            .child(
                                                                div()
                                                                    .flex_shrink_0()
                                                                    .w(px(34.0))
                                                                    .h(px(34.0))
                                                                    .rounded_full()
                                                                    .bg(rgb(p.claude))
                                                                    .text_color(rgb(0xFFFFFF))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .text_sm()
                                                                    .font_weight(FontWeight::BOLD)
                                                                    .child("✦"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id("bubble_typing_indicator")
                                                                    .flex_1()
                                                                    .min_w_0()
                                                                    .text_size(px(cur_font_size))
                                                                    .px_4()
                                                                    .py_3()
                                                                    .rounded_2xl()
                                                                    .rounded_tl_sm()
                                                                    .bg(rgb(p.card))
                                                                    .border_1()
                                                                    .border_color(rgba(p.claude_alpha(0x66)))
                                                                    .shadow_sm()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_3()
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .font_weight(FontWeight::BOLD)
                                                                            .text_color(rgb(p.claude))
                                                                            .child("✦ Claude Code 正在思考回复中..."),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(rgb(p.muted))
                                                                            .child("⠋⠙⠹⠸"),
                                                                    ),
                                                            ),
                                                    )
                                                    .into_any_element(),
                                            );
                                        }

                                        items
                                    }),
                            )
                            // ─────────────────────────────────────────────────
                            // 【右侧下方】：固定对话框 + Tokens + 配额用量 + 模型切换 (flex_shrink_0)
                            // ─────────────────────────────────────────────────
                            // Fixed Chat & Quota Bar Area (Transparent Background & Left Text Layout)
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .px_6()
                                    .pb_4()
                                    .pt_1()
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .w_full()
                                            .max_w(px(740.0))
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            // 1. 固定的对话框 (Fixed Chat Box - 完全透明无背景色)
                                            .child(
                                                div()
                                                    .rounded_xl()
                                                    .border_1()
                                                    .border_color(rgb(p.border))
                                                    .hover(|s| s.border_color(rgba(p.claude_alpha(0x99))))
                                                    .p_3()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_2()
                                                    .child(
                                                        Input::new(&self.prompt_input)
                                                            .appearance(false)
                                                            .with_size(ComponentSize::Small),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .pt_1()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(rgb(p.muted))
                                                                    .child("Enter ↵ 提交 · Shift+Enter 换行"),
                                                            )
                                                            .child(
                                                                Button::new("btn_send_prompt")
                                                                    .primary()
                                                                    .with_size(ComponentSize::XSmall)
                                                                    .label(if self.is_generating { "⠋ 思考中..." } else { "发送 ↵" })
                                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                                        this.submit_prompt(window, cx);
                                                                    })),
                                                            ),
                                                    ),
                                            )
                                            // 2. 对话栏左下角单行纯文字状态信息 (Tokens、5小时剩余、周剩余)
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .px_1()
                                                    .text_xs()
                                                    .whitespace_nowrap()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_1p5()
                                                            .whitespace_nowrap()
                                                            .flex_shrink_0()
                                                            .text_color(rgb(p.muted))
                                                            .child(
                                                                div()
                                                                    .whitespace_nowrap()
                                                                    .child(format!("Tokens: {:.1}%", used_pct)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_color(rgba(p.muted_alpha(0x66)))
                                                                    .whitespace_nowrap()
                                                                    .child("|"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_0p5()
                                                                    .whitespace_nowrap()
                                                                    .flex_shrink_0()
                                                                    .child(
                                                                        div()
                                                                            .whitespace_nowrap()
                                                                            .child("5h余: "),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_color(rgb(p.green))
                                                                            .font_weight(FontWeight::BOLD)
                                                                            .whitespace_nowrap()
                                                                            .child(format!("{:.1}%", five_h_pct)),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .whitespace_nowrap()
                                                                            .child(format!(" ({})", five_h_reset)),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_color(rgba(p.muted_alpha(0x66)))
                                                                    .whitespace_nowrap()
                                                                    .child("|"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_0p5()
                                                                    .whitespace_nowrap()
                                                                    .flex_shrink_0()
                                                                    .child(
                                                                        div()
                                                                            .whitespace_nowrap()
                                                                            .child("周余: "),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_color(rgb(p.green))
                                                                            .font_weight(FontWeight::BOLD)
                                                                            .whitespace_nowrap()
                                                                            .child(format!("{:.1}%", weekly_pct)),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .whitespace_nowrap()
                                                                            .child(format!(" ({})", weekly_reset)),
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new("btn_bottom_model_selector")
                                                            .ghost()
                                                            .with_size(ComponentSize::XSmall)
                                                            .label(format!("🤖 {} ▾", self.selected_model))
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.show_model_menu = !this.show_model_menu;
                                                                cx.notify();
                                                            })),
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
            )
            // ─────────────────────────────────────────────────────────────
            // 4. 全局浮层：模型切换选择器 (Top-Level Model Popover Menu)
            // ─────────────────────────────────────────────────────────────
            .when(show_model_menu, |this| {
                let cur_active = cur_model.clone();

                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x00000044))
                        .flex()
                        .items_end()
                        .justify_end()
                        .p_6()
                        .child(
                            div()
                                .w(px(460.0))
                                .p_4()
                                .rounded_2xl()
                                .bg(rgb(p.card))
                                .border_1()
                                .border_color(rgb(p.claude))
                                .shadow_xl()
                                .flex()
                                .flex_col()
                                .gap_2p5()
                                // Header
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .pb_2()
                                        .border_b_1()
                                        .border_color(rgb(p.border))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(p.claude))
                                                .child("✦ 切换当前活跃模型 (Select Model)"),
                                        )
                                        .child(
                                            Button::new("btn_close_model_popover")
                                                .ghost()
                                                .with_size(ComponentSize::Small)
                                                .label("✕ 关闭")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.show_model_menu = false;
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                // Model list
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .children(AVAILABLE_MODELS.iter().enumerate().map(|(idx, m)| {
                                            let is_selected = cur_active == m.name;
                                            let model_name = m.name.to_string();

                                            div()
                                                .id(format!("model_card_opt_{}", idx))
                                                .p_2p5()
                                                .rounded_lg()
                                                .cursor(CursorStyle::PointingHand)
                                                .bg(if is_selected { rgb(p.card_active) } else { rgba(0x00000000) })
                                                .border_1()
                                                .border_color(if is_selected { rgb(p.claude) } else { rgb(p.border) })
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.switch_model(model_name.clone(), cx);
                                                }))
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(if is_selected { rgb(p.claude) } else { rgb(p.text) })
                                                                .child(format!("{}{}", if is_selected { "✓ " } else { "  " }, m.name)),
                                                        )
                                                        .child(
                                                            div()
                                                                .px_2()
                                                                .py(px(1.0))
                                                                .rounded_sm()
                                                                .bg(rgba(p.claude_alpha(0x22)))
                                                                .text_xs()
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(rgb(p.claude))
                                                                .child(m.tag),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(p.muted))
                                                        .child(m.desc),
                                                )
                                                .into_any_element()
                                        })),
                                ),
                        ),
                )
            })
            // ─────────────────────────────────────────────────────────────
            // 5. 设置界面 (Settings Modal - 第二个界面)
            // ─────────────────────────────────────────────────────────────
            .when(show_settings, |this| {
                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x00000088))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(780.0))
                                .h(px(590.0))
                                .rounded_2xl()
                                .bg(rgb(p.card))
                                .border_1()
                                .border_color(rgb(p.border))
                                .shadow_xl()
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                // Settings Header Bar
                                .child(
                                    div()
                                        .h(px(52.0))
                                        .px_5()
                                        .bg(rgb(p.sidebar))
                                        .border_b_1()
                                        .border_color(rgb(p.border))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2p5()
                                                .child(
                                                    div()
                                                        .text_base()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(rgb(p.text))
                                                        .child("⚙️ Antigravity CLI 设置中心"),
                                                )
                                                .child(
                                                    div()
                                                        .px_2()
                                                        .py(px(1.0))
                                                        .rounded_sm()
                                                        .bg(rgba(p.claude_alpha(0x22)))
                                                        .text_xs()
                                                        .text_color(rgb(p.claude))
                                                        .child("Preferences"),
                                                ),
                                        )
                                        .child(
                                            Button::new("btn_close_settings_top")
                                                .ghost()
                                                .with_size(ComponentSize::Small)
                                                .label("✕")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.show_settings = false;
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                // Settings Main Body (Left Tabs + Right Config View)
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_row()
                                        .overflow_hidden()
                                        // Left Navigation Tabs
                                        .child(
                                            div()
                                                .w(px(190.0))
                                                .h_full()
                                                .bg(rgb(p.sidebar))
                                                .border_r_1()
                                                .border_color(rgb(p.border))
                                                .p_3()
                                                .flex()
                                                .flex_col()
                                                .gap_2()
                                                .child(
                                                    Button::new("tab_btn_0")
                                                        .with_variant(if current_tab == 0 { ButtonVariant::Primary } else { ButtonVariant::Ghost })
                                                        .with_size(ComponentSize::Small)
                                                        .label("🎨 外观与排版")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.settings_tab = 0;
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    Button::new("tab_btn_1")
                                                        .with_variant(if current_tab == 1 { ButtonVariant::Primary } else { ButtonVariant::Ghost })
                                                        .with_size(ComponentSize::Small)
                                                        .label("🤖 模型与智能体")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.settings_tab = 1;
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    Button::new("tab_btn_2")
                                                        .with_variant(if current_tab == 2 { ButtonVariant::Primary } else { ButtonVariant::Ghost })
                                                        .with_size(ComponentSize::Small)
                                                        .label("💻 终端集成")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.settings_tab = 2;
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    Button::new("tab_btn_3")
                                                        .with_variant(if current_tab == 3 { ButtonVariant::Primary } else { ButtonVariant::Ghost })
                                                        .with_size(ComponentSize::Small)
                                                        .label("📁 路径与关于")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.settings_tab = 3;
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                        // Right Config Area
                                        .child(
                                            div()
                                                .id("settings_content_view")
                                                .flex_1()
                                                .h_full()
                                                .overflow_y_scroll()
                                                .p_5()
                                                .flex()
                                                .flex_col()
                                                .gap_4()
                                                .children(match current_tab {
                                                    // 0: 外观与排版 (外观、色彩、聊天字号独立缩放)
                                                    0 => vec![
                                                        div().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("界面外观与色彩选项"),
                                                        // 1. 主题色彩模式
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("主题色彩模式 (自动永久记忆)")).child(div().text_xs().text_color(rgb(p.muted)).child("切换 Claude 经典暖暗色或明亮浅色主题")))
                                                            .child(Button::new("btn_set_theme").primary().with_size(ComponentSize::Small).label(if is_dark_mode { "🌙 当前: 暖暗色 (Dark)" } else { "☀️ 当前: 浅色 (Light)" }).on_click(cx.listener(|this, _, _, cx| { this.toggle_theme(cx); }))),
                                                        // 2. 聊天气泡字体大小 (Telegram 独立字号缩放体系)
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().flex_col().gap_2p5()
                                                            .child(
                                                                div().flex().items_center().justify_between()
                                                                    .child(
                                                                        div().flex().flex_col().gap_0p5()
                                                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("聊天气泡字体大小 (Chat Text Scale)"))
                                                                            .child(div().text_xs().text_color(rgb(p.muted)).child("独立缩放聊天区域 · 快捷键: Ctrl + 放大 / Ctrl - 缩小 / Ctrl 0 恢复"))
                                                                    )
                                                                    .child(
                                                                        div().flex().items_center().gap_1p5()
                                                                            .child(
                                                                                Button::new("btn_set_font_sub")
                                                                                    .secondary()
                                                                                    .with_size(ComponentSize::XSmall)
                                                                                    .label("－ 缩小")
                                                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                                                        this.zoom_out_font(cx);
                                                                                    }))
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .px_3()
                                                                                    .py_1()
                                                                                    .rounded_md()
                                                                                    .bg(rgba(p.claude_alpha(0x22)))
                                                                                    .text_xs()
                                                                                    .font_weight(FontWeight::BOLD)
                                                                                    .text_color(rgb(p.claude))
                                                                                    .child(format!("{:.1}px", cur_font_size))
                                                                            )
                                                                            .child(
                                                                                Button::new("btn_set_font_add")
                                                                                    .secondary()
                                                                                    .with_size(ComponentSize::XSmall)
                                                                                    .label("＋ 放大")
                                                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                                                        this.zoom_in_font(cx);
                                                                                    }))
                                                                            )
                                                                            .child(
                                                                                Button::new("btn_set_font_reset")
                                                                                    .ghost()
                                                                                    .with_size(ComponentSize::XSmall)
                                                                                    .label("⟲ 默认 14px")
                                                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                                                        this.reset_zoom_font(cx);
                                                                                    }))
                                                                            )
                                                                    )
                                                            )
                                                            // 快捷档位预设 (Presets)
                                                            .child(
                                                                div().flex().items_center().gap_2()
                                                                    .child(
                                                                        Button::new("preset_12_5")
                                                                            .with_variant(if (cur_font_size - 12.5).abs() < 0.1 { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("12.5px 极客紧凑")
                                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                                this.set_chat_font_size(12.5, cx);
                                                                            }))
                                                                    )
                                                                    .child(
                                                                        Button::new("preset_14_0")
                                                                            .with_variant(if (cur_font_size - 14.0).abs() < 0.1 { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("14.0px 标准 (推荐)")
                                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                                this.set_chat_font_size(14.0, cx);
                                                                            }))
                                                                    )
                                                                    .child(
                                                                        Button::new("preset_15_5")
                                                                            .with_variant(if (cur_font_size - 15.5).abs() < 0.1 { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("15.5px 舒适阅读")
                                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                                this.set_chat_font_size(15.5, cx);
                                                                            }))
                                                                    )
                                                                    .child(
                                                                        Button::new("preset_17_0")
                                                                            .with_variant(if (cur_font_size - 17.0).abs() < 0.1 { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                                                                            .with_size(ComponentSize::XSmall)
                                                                            .label("17.0px 大号")
                                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                                this.set_chat_font_size(17.0, cx);
                                                                            }))
                                                                    )
                                                            )
                                                            // 实时排版效果预览卡片
                                                            .child(
                                                                div()
                                                                    .p_2p5()
                                                                    .rounded_lg()
                                                                    .bg(rgb(p.sidebar))
                                                                    .border_1()
                                                                    .border_color(rgba(p.claude_alpha(0x33)))
                                                                    .text_size(px(cur_font_size))
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap_1p5()
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(rgb(p.muted))
                                                                            .child(format!("💬 实时效果预览 ({:.1}px):", cur_font_size))
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .p_2()
                                                                            .rounded_lg()
                                                                            .bg(rgb(p.card))
                                                                            .border_1()
                                                                            .border_color(rgb(p.border))
                                                                            .text_color(rgb(p.text))
                                                                            .child("✦ Claude: 当前字号排版紧凑优雅，多行文本阅读更自然。")
                                                                    )
                                                            ),
                                                        // 3. 默认系统字体
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("系统默认字体")).child(div().text_xs().text_color(rgb(p.muted)).child(format!("全界面统一跟随系统字体：{}", self.system_font))))
                                                            .child(div().px_3().py_1().rounded_md().bg(rgba(p.claude_alpha(0x22))).text_xs().text_color(rgb(p.claude)).child(self.system_font.clone())),
                                                    ],
                                                    // 1: 模型与智能体
                                                    1 => vec![
                                                        div().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("Antigravity 智能体运行配置"),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("当前主模型")).child(div().text_xs().text_color(rgb(p.muted)).child("点击右侧按钮可直接弹出菜单切换")))
                                                            .child(Button::new("btn_settings_change_model").primary().with_size(ComponentSize::Small).label(format!("✦ 切换模型: {} ▾", cur_model)).on_click(cx.listener(|this, _, _, cx| { this.show_model_menu = true; cx.notify(); }))),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("自动通过工具权限 (Skip Permissions)")).child(div().text_xs().text_color(rgb(p.muted)).child("唤起 CLI 时是否携带 --dangerously-skip-permissions")))
                                                            .child(Button::new("btn_toggle_perm").with_variant(if skip_perm { ButtonVariant::Primary } else { ButtonVariant::Secondary }).with_size(ComponentSize::Small).label(if skip_perm { "已开启" } else { "已禁用" }).on_click(cx.listener(|this, _, _, cx| {
                                                                this.auto_skip_permissions = !this.auto_skip_permissions;
                                                                this.status_msg = Some(format!("已{}危险权限自动跳过", if this.auto_skip_permissions { "开启" } else { "关闭" }));
                                                                cx.notify();
                                                            }))),
                                                    ],
                                                    // 2: 终端集成
                                                    2 => vec![
                                                        div().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("终端模拟器切入配置"),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("首选终端")).child(div().text_xs().text_color(rgb(p.muted)).child("检测系统 Alacritty / Konsole")))
                                                            .child(div().px_3().py_1().rounded_md().bg(rgba(p.green_alpha(0x22))).text_xs().text_color(rgb(p.green)).child("✓ Alacritty (已就绪)")),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().items_center().justify_between()
                                                            .child(div().flex().flex_col().gap_0p5().child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("唤起参数")).child(div().text_xs().text_color(rgb(p.muted)).child("支持无缝切入指定 conversation_id")))
                                                            .child(div().px_3().py_1().rounded_md().bg(rgb(p.sidebar)).text_xs().text_color(rgb(p.muted)).child("agy --conversation <cid>")),
                                                    ],
                                                    // 3: 路径与关于
                                                    _ => vec![
                                                        div().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("Antigravity 路径与环境"),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().flex_col().gap_1()
                                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("CLI 配置主目录"))
                                                            .child(div().text_xs().text_color(rgb(p.muted)).child("~/.gemini/antigravity-cli")),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().flex_col().gap_1()
                                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("会话数据库 (SQLite)"))
                                                            .child(div().text_xs().text_color(rgb(p.muted)).child("~/.gemini/antigravity-cli/conversation_summaries.db")),
                                                        div().p_3().rounded_lg().bg(rgb(p.card)).border_1().border_color(rgb(p.border)).flex().flex_col().gap_1()
                                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(p.text)).child("GUI 引擎版本"))
                                                            .child(div().text_xs().text_color(rgb(p.claude)).child("gpui-kit 0.9.0 (Zed GPU Native Framework)")),
                                                    ],
                                                }.into_iter().map(|el| el.into_any_element()).collect::<Vec<AnyElement>>()),
                                        ),
                                )
                                // Settings Bottom Action Bar
                                .child(
                                    div()
                                        .h(px(52.0))
                                        .px_5()
                                        .bg(rgb(p.sidebar))
                                        .border_t_1()
                                        .border_color(rgb(p.border))
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .child(
                                            Button::new("btn_done_settings")
                                                .primary()
                                                .with_size(ComponentSize::Small)
                                                .label("完成设置并返回")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.show_settings = false;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        ),
                )
            })
            // ─────────────────────────────────────────────────────────────
            // 6. 重命名对话框 (Rename Modal)
            // ─────────────────────────────────────────────────────────────
            .when_some(self.renaming_item.clone(), |this, item| {
                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x000000AA))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(460.0))
                                .p_5()
                                .rounded_2xl()
                                .bg(rgb(p.card))
                                .border_1()
                                .border_color(rgb(p.claude))
                                .shadow_xl()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(p.claude))
                                        .child("重命名会话"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(p.muted))
                                        .child(format!("当前会话 UUID: {}", item.id)),
                                )
                                .child(Input::new(&self.rename_input).with_size(ComponentSize::Small))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap_2()
                                        .child(
                                            Button::new("btn_cancel_rename")
                                                .ghost()
                                                .with_size(ComponentSize::Small)
                                                .label("取消")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.cancel_rename(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("btn_confirm_rename")
                                                .primary()
                                                .with_size(ComponentSize::Small)
                                                .label("保存")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.save_rename(cx);
                                                })),
                                        ),
                                ),
                        ),
                )
            })
            // ─────────────────────────────────────────────────────────────
            // 7. 删除确认对话框 (Delete Modal)
            // ─────────────────────────────────────────────────────────────
            .when_some(self.deleting_item.clone(), |this, item| {
                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x000000AA))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(460.0))
                                .p_5()
                                .rounded_2xl()
                                .bg(rgb(p.card))
                                .border_1()
                                .border_color(rgb(p.claude))
                                .shadow_xl()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(p.claude))
                                        .child("确认删除此会话？"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(p.text))
                                        .child(format!("即将删除：【{}】", item.title)),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(p.muted))
                                        .child("此操作将清理本地关联数据库记录、产物文件夹与锁文件，不可恢复。"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap_2()
                                        .child(
                                            Button::new("btn_cancel_delete")
                                                .ghost()
                                                .with_size(ComponentSize::Small)
                                                .label("取消")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.cancel_delete(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("btn_confirm_delete")
                                                .danger()
                                                .with_size(ComponentSize::Small)
                                                .label("彻底删除")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_delete(cx);
                                                })),
                                        ),
                                ),
                        ),
                )
            })
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);

            // Load saved theme preference on start with system font
            let is_dark = session::load_theme_preference();
            let sys_font = detect_system_font();
            Theme::change(if is_dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
            Theme::update(cx, |theme| {
                theme.font_family = sys_font.clone().into();
                theme.mono_font_family = sys_font.into();
            });

            // Window options: Modern desktop window with standard rectangular appearance
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1240.0), px(820.0)),
                    cx,
                ))),
                titlebar: None,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
                ..Default::default()
            };

            gpui_kit::open_window(window_options, cx, |window, cx| {
                cx.new(|cx| AntiCliApp::new(window, cx))
            })
            .expect("Failed to open Claude Code window");
        });
}
