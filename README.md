# AntiCliUI (Agy UI)

AntiCliUI 是专为 Antigravity (Claude Code CLI) 打造的高性能现代化桌面客户端，基于 Rust 原生 GPU 渲染框架 **GPUI** 构建。提供类似 Telegram / Claude 的丝滑气泡交互体验、纯净的提纯问答阅读与无干扰的直接对话体验。

---

## 核心特性

- **直接内嵌对话 & 后台异步执行**：
  - 支持在客户端输入框直接与 Antigravity / Claude 对话，无需繁琐弹出外部终端。
  - 携带实时打字思考指示器 (`✦ Claude Code 正在思考回复中... ⠋⠙⠹⠸`)，回答完成后即时渲染与更新会话。
- **纯净问答模式（Clean Turn Filtering）**：
  - 自动提纯对话上下文，仅展示用户的核心提问与 Claude 的最终回答。
  - 隐去中间冗长的思考过程（Thinking）、工具调用详情与冗余系统信息，专注阅读与生产力。
- **严格 48% 中线约束气泡布局**：
  - 用户气泡严格限制在右侧半屏（52% ~ 100%），AI 气泡严格限制在左侧半屏（0% ~ 48%）。
  - 双侧绝对不越过中央 50% 分割线，中间保留 4% 呼吸留白。
- **独立聊天字号无级缩放与设置**：
  - 独立的聊天文本缩放体系，与全局 UI 分离，支持在设置面板中直接调节。
  - 提供 4 档快捷预设：`12.5px 极客紧凑`、`14.0px 标准 (推荐)`、`15.5px 舒适阅读`、`17.0px 大号`。
  - 全局快捷键实时缩放：`Ctrl +` / `Ctrl =` 放大，`Ctrl -` 缩小，`Ctrl 0` 恢复默认。
- **细节打磨 & Telegram 质感**：
  - AI 气泡时间显示靠左紧凑排列，避免双行折行。
  - 气泡右下角配备轻量一键复制按钮（图标化无文字干扰）。
  - 移除已读对钩等冗余元素，界面纯净通透。
- **极简顶栏状态**：
  - 右侧上方仅保留当前会话标题与活跃状态徽标 (`● Active` / `○ Inactive`)，剔除杂乱信息。
- **实时用量与配额监控**：
  - 自动同步展示当前会话 Token 消耗、5 小时滑动窗口限额及每周限额进度。

---

## 架构说明

- **语言框架**：Rust + [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (`gpui-kit`)
- **数据源**：直接读取系统 Antigravity / Claude 对话记录与 transcript 索引数据库。
- **执行器**：异步后台调用系统 `agy` CLI，确保主界面 120 FPS 流畅无卡顿。

---

## 快捷键一览

| 快捷键 | 功能描述 |
| :--- | :--- |
| `Ctrl +` / `Ctrl =` | 放大聊天区文字字号 |
| `Ctrl -` | 缩小聊天区文字字号 |
| `Ctrl 0` | 恢复聊天文字至默认字号 (14.0px) |
| `Enter` (在单行输入框) | 发送消息并后台调用 Antigravity |

---

## 快速上手与编译

### 前置要求

- Rust 1.80+ (推荐使用 rustup 安装)
- Linux 系统依赖库（X11 / Wayland, fontconfig, freetype 等，GPUI 基础编译环境）

### 编译与运行

```bash
# 1. 克隆代码仓库
git clone git@github.com:lopree/AntiCliUI.git
cd AntiCliUI

# 2. 编译 Release 版本
cargo build --release

# 3. 运行体验
./target/release/anti_cli_ui
```

### 安装至系统路径

```bash
cp target/release/anti_cli_ui ~/.local/bin/agy-ui
```

即可在系统终端或应用启动器中随时输入 `agy-ui` 唤起使用。

---

## 开源协议

MIT License
