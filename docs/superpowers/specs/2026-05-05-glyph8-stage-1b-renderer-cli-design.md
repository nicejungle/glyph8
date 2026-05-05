# Glyph8 — Stage 1.B Renderer + CLI Beta 设计

**目标**：在 Stage 0 + 1.A（`nes-core` + `nes-tetanes-backend` 库已就绪）基础上，新增 `nes-render` 与 `glyph8-cli` 两个 crate，让用户能在终端里加载 ROM、看见画面、用键盘控制游戏，作为可发布的 **beta 版本**。本阶段不做音频（1.D）、不做 Kitty 输入协议（1.C）、不做 braille/ascii 渲染（1.E）、不做 ratatui 状态栏暂停 UI（1.F）。

**端到端验收**：`glyph8 path/to/rom.nes` 启动 → halfblock 终端画面跑起来 → 键盘可控 → `Esc` 退出且终端状态完整恢复。`--headless --frames=N` 给 CI 做 frame buffer 哈希比对。

## 1. 范围与非范围

**包含**：
- 新 crate `nes-render`：极简 `Renderer` trait + `HalfblockRenderer` 实现
- 新 crate `glyph8-cli`：`glyph8` 二进制、交互模式主循环、headless 模式
- 基础键盘输入（crossterm，方向键 + Z/X/Enter/RShift + R + Esc）
- 仓库自带测试 ROM（nestest + 1–2 个 CC0/PD 同人 demo）
- 端到端 headless 集成测试

**不包含（明确延后）**：
- 音频（→ 1.D）
- Kitty keyboard protocol（→ 1.C，beta 用普通 crossterm 就够测试）
- braille / ascii 渲染（→ 1.E）
- ratatui 状态栏 / 暂停模态（→ 1.F；本阶段只画一行底部文字）
- 自适应降色（→ 1.E；本阶段假设终端支持真彩色）
- P2 手柄（→ spec §14 已记 YAGNI）
- 即时存档 / 录像 / 调试器
- CI（GitHub Actions yaml 留到 1.F 或更后；本阶段保证本地测试绿即可）

## 2. 架构

```
crates/
├── nes-core/                    （已有；Frame, ControllerState, Backend trait）
├── nes-tetanes-backend/         （已有；TetanesBackend）
├── nes-render/                  新增
│   └── src/
│       ├── lib.rs               Renderer trait（3 个方法）
│       └── halfblock.rs         HalfblockRenderer 实现
└── glyph8-cli/                  新增（[[bin]] name = "glyph8"）
    └── src/
        ├── main.rs              clap 参数 + 模式分发
        ├── runloop.rs           交互模式主循环
        ├── input.rs             crossterm 键事件 → ControllerState + 控制事件
        └── headless.rs          --headless 模式
```

**依赖关系**：
- `nes-render` → `nes-core`（仅 `Frame`），不依赖任何 backend
- `glyph8-cli` → `nes-core` + `nes-tetanes-backend` + `nes-render`

**为什么现在就抽 `Renderer` trait**：spec §5 已指定；trait 三个方法，写出来比 1.E 时返工小得多。1.B 只实现 `HalfblockRenderer`，braille / ascii 留到 1.E 直接 `impl Renderer`。

## 3. 关键组件

### 3.1 `nes-render::Renderer`

```rust
use std::io;
use nes_core::Frame;

pub trait Renderer {
    /// 进入终端备用屏 + 切 raw 模式 + 隐藏光标。
    fn enter(&mut self) -> io::Result<()>;
    /// 把一帧画到终端。调用方保证 `frame.pixels.len() == FRAME_BYTES`。
    fn draw(&mut self, frame: &Frame) -> io::Result<()>;
    /// 退出备用屏 + 恢复光标 + 关 raw。`Drop` 也要 best-effort 调一次。
    fn leave(&mut self) -> io::Result<()>;
}
```

设计取舍：trait 故意小。状态栏不进 trait（不同渲染器对"额外文字行"的实现差距太大），由 `glyph8-cli` 在 `draw` 后单独 print 一行。

### 3.2 `HalfblockRenderer`

- 字符：`▀` (U+2580 上半块)
- 编码：每个 cell = 2 个像素（fg = 上像素 RGB，bg = 下像素 RGB）
- 屏幕：256 NES 列 × 240 NES 行 → 256 终端列 × 120 终端行（高度对半）
- 写入：`crossterm::queue!` 批量进 stdout buffer，一帧末尾 `flush`
- **diff 重画**：维护 `prev: Vec<(fg, bg)>`，仅对变化的 cell 写"光标定位 + ANSI 24-bit 颜色 + ▀"。理由：全屏重画 30,720 cell × 60 fps 实测会卡（~70 MB/s stdout），diff 必须从第一版上
- `Drop` impl 调 `leave()`，panic 时也保证终端恢复

### 3.3 `glyph8-cli::Input`

```rust
pub enum PollOutcome {
    Continue(ControllerState),  // p1 当前持有的按键集合
    Reset,
    Quit,
}

pub struct Input { p1: ControllerState }
impl Input {
    pub fn new() -> io::Result<Self>;
    pub fn poll(&mut self) -> io::Result<PollOutcome>;
}
```

- 用 `crossterm::event::poll(Duration::ZERO)` 非阻塞读所有 pending 事件
- 维护内部 `ControllerState`：`KeyEvent { kind: Press }` 调 `press(bit)`，`Release` 调 `release(bit)`
- crossterm 默认不报 release 事件（除非启用 `KeyboardEnhancementFlags::REPORT_EVENT_TYPES`），1.B 简化为：每帧开始前 `release_all`，再把当前 frame 内所有 `Press` 重新 set。妥协：键按住时模拟器只会看到"按下→松开→按下…"的鼠标式输入。对大部分游戏（马里奥跑跳、对话推进）够用；不够用就在 1.C Kitty 协议时修。这个 trade-off 在 README 里写明
- 键位映射：

| 键 | 动作 |
|---|---|
| ↑ ↓ ← → | NES Up / Down / Left / Right |
| Z | NES B |
| X | NES A |
| Enter | NES Start |
| RightShift | NES Select |
| R | Reset（emulator） |
| Esc / Ctrl+C | Quit |

### 3.4 `glyph8-cli::runloop`

```rust
pub fn run(rom_bytes: &[u8]) -> anyhow::Result<()> {
    let mut backend = TetanesBackend::new();
    backend.load_rom(rom_bytes)?;

    let mut renderer = HalfblockRenderer::new()?;
    let mut input = Input::new()?;
    renderer.enter()?;

    let frame_dur = Duration::from_nanos(16_639_267); // NTSC 60.0988 Hz
    let mut next = Instant::now() + frame_dur;
    loop {
        match input.poll()? {
            PollOutcome::Quit => break,
            PollOutcome::Reset => backend.reset(),
            PollOutcome::Continue(p1) => backend.submit_input(p1, ControllerState::empty()),
        }
        backend.step_frame()?;
        renderer.draw(backend.frame())?;
        write_status_line(&backend, fps_meter.tick())?;
        let now = Instant::now();
        if next > now { std::thread::sleep(next - now); }
        next += frame_dur; // 落帧不补偿；下帧 deadline 自然继续
    }
    renderer.leave()?;
    Ok(())
}
```

状态栏：`<rom_name> | FPS: <measured> | ESC: quit | R: reset`，一行，写在画面下方。FPS 用滑动窗口（最近 60 帧均值）。

### 3.5 `glyph8-cli::headless`

```rust
pub fn run(rom_bytes: &[u8], frames: u32) -> anyhow::Result<()> {
    let mut backend = TetanesBackend::new();
    backend.load_rom(rom_bytes)?;
    for _ in 0..frames {
        backend.step_frame()?;
    }
    let hash = blake3::hash(&backend.frame().pixels);
    println!("{}", hash.to_hex());
    Ok(())
}
```

退出码 0 = 正常，非 0 = `step_frame` / `load_rom` 出错。CI 用它做黄金哈希比对。

### 3.6 CLI 参数

clap derive：

```
glyph8 [OPTIONS] <ROM>

Args:
  <ROM>                  Path to .nes file

Options:
      --headless         Run N frames headless and print frame hash
      --frames <N>       Frames to step in headless mode [default: 60]
                         (Ignored without --headless; not an error)
  -h, --help
  -V, --version
```

## 4. 数据流 & 生命周期

启动：parse_args → 读 ROM 文件 → bytes → 模式分发（headless 或 runloop）。

交互模式单次迭代：

```
input.poll()                  ┐
  → Quit / Reset / Continue   │
backend.submit_input(p1, _)   │
backend.step_frame()?         │  约 16.64 ms 一轮
renderer.draw(frame)?         │
write_status_line()?          │
sleep_until(next_deadline)    ┘
```

退出（Quit / Err / panic）：`Drop` 链 Input → Renderer 恢复终端，错误 stderr，退出码 0/1。

## 5. 错误处理

| 失败点 | 处理 |
|---|---|
| ROM 文件读不到 | `eprintln!` + 退出 1，未进入 raw 模式所以无需恢复 |
| iNES 解析失败（`EmulatorError::InvalidRom`） | 同上 |
| `backend.step_frame()` 返回 `Err` | 主循环 break，Drop 链恢复终端，`eprintln!`，退出 1 |
| Renderer io error | 同 step_frame |
| 终端不支持真彩色 | 1.B 不检测；用户看到颜色错位即降级到 1.E 处理 |
| Panic | `Drop` 链确保终端恢复；标准 panic 输出已够 |

约定：`anyhow::Result` 仅 `glyph8-cli` 内部用；`nes-render` 仍用 `io::Result`，保持库纯净。

## 6. 测试策略

| 层 | 测什么 | 怎么测 |
|---|---|---|
| `nes-render` 单测 | halfblock 编码：简单 Frame → 期望 ANSI 字节前缀 | `Vec<u8>` 替代 stdout |
| `nes-render` golden | 已知 Frame → ANSI 字节流的 blake3 哈希等于 fixture | `tests/halfblock_golden.rs` |
| `nes-render` diff | 第二次 draw 同一 Frame 应产生最小输出（仅 cursor reset 之类） | 字节数比首次 < 5% |
| `glyph8-cli` 集成 | `--headless --frames=60 nestest.nes` → frame hash == 已记录黄金值 | `tests/headless_nestest.rs` 用 `Command::new(env!("CARGO_BIN_EXE_glyph8"))` |
| 手动 QA | 跑同人 demo，肉眼验收画面 + 输入响应 | `docs/qa-checklist.md` 增补 1.B 节 |

CI 不立。本阶段以 `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` 在本地通过为准。

## 7. 测试 ROM 策略

仓库内 `tests/roms/`：

- **`nestest.nes`** — 公共领域，CPU 指令验证。来源：`http://nickmass.com/images/nestest.nes`（社区共识公共领域，每个 NES 模拟器项目都用）
- **同人 demo × 1–2 个** — Plan Task 0 由实施者研究，候选条件：
  - 许可证：CC0 / public domain / 作者明确允许重分发
  - 体积：≤ 100 KB 单 ROM，便于入仓
  - 启动后能产生明显画面变化（不是黑屏 demo）
  - 候选示例：NESdev wiki 自带的 minimal demos、Lizard demo 的免费章节、`nesdev_compo` 历年公共条目
  - 实施者在 Plan Task 0 给出 2–3 个候选 + 出处链接，由用户拍板再下载入库

商业 ROM（马里奥、塞尔达等）**不进仓库**；用户运行时 `glyph8 path/to/their.nes` 自带。README 写明法律边界。

## 8. 依赖增量

| crate | 新依赖 |
|---|---|
| `nes-render` | `crossterm` |
| `glyph8-cli` | `clap` (derive)、`anyhow`、`blake3`、`crossterm`（间接，从 nes-render） |

依赖增量受控：4 个新 crate-level 依赖，全部主流。

## 9. 自我评审

**针对 spec 设计文档（§5、§11、§14）的覆盖**：

| spec 项 | 覆盖于 |
|---|---|
| §5 Renderer trait | §3.1 |
| §5 halfblock 默认 | §3.2 |
| §5 braille / ascii 延后 | §1 明确延后到 1.E |
| §6 输入抽象 + Kitty 协议 | §3.3（Kitty 延后到 1.C，beta 用 crossterm 基础事件） |
| §11 `glyph8 <rom>` CLI | §3.6 |
| §11 `--render=` flag | 延后；1.B 只有 halfblock |
| §11 `--backend=` flag | 延后；1.B 只有 tetanes，无第二选择 |
| §14 P2 手柄 YAGNI | 保持 YAGNI |

**已知偏离**：
- spec §11 的完整 CLI 长这样 `glyph8 --render=braille --backend=native ...`。1.B 只暴露 `<ROM>` + `--headless` + `--frames`，其他 flag 留给后续 stage 接通时增补。理由：现在加 `--render` flag 但只能填 `halfblock` 是死代码。
- spec §6 设计的输入抽象层（`InputSource` trait + Kitty/crossterm 两实现）在 1.B 简化为 `glyph8-cli::Input` 单一结构体。理由：beta 只一种输入方式，trait 是 1.C 的事。
- spec 没明确规定状态栏；1.B 实现一行简陋文字版，留给 1.F 升级到 ratatui。

## 10. 路线对接

- **下一个 plan**：`docs/superpowers/plans/2026-05-05-glyph8-stage-1b.md`（实施计划，TDD 任务粒度）
- **完成定义**：
  - `cargo test --workspace` 全绿
  - `cargo clippy --workspace --all-targets -- -D warnings` 无警告
  - `cargo run -p glyph8-cli -- tests/roms/nestest.nes` 能跑起来、ESC 能干净退出
  - `cargo run -p glyph8-cli -- --headless --frames=60 tests/roms/nestest.nes` 打印一致 hash
  - `docs/qa-checklist.md` 增加 1.B 节
- **承接 1.C**：把 `Input` 抽成 trait，加 Kitty 协议实现
- **承接 1.D**：在 runloop 内插入音频管道（cpal sink + tetanes audio_samples）
- **承接 1.E**：`HalfblockRenderer` 旁边加 `BrailleRenderer` / `AsciiRenderer`，`--render=` flag 接通

---

**End of design.**
