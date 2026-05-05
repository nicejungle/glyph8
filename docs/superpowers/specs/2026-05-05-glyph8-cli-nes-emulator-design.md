# Glyph8 — CLI 红白机模拟器 设计文档

- **日期**：2026-05-05
- **作者**：与 Claude Code（superpowers/brainstorming）协作
- **状态**：已批准，待出实现计划

## 1. 目标与范围

构建一个跑在终端里的 NES（Famicom / 红白机）模拟器，名为 **Glyph8**（"Glyph" = 字符，"8" = 8-bit）。

差异化卖点：完全在命令行里游玩 NES 游戏，画面通过半块字符 / Braille / ASCII 渲染到终端。

### 项目分两个阶段

- **阶段 1（MVP-中）**：复用现成模拟器核心 [`tetanes`](https://crates.io/crates/tetanes)，专注做 CLI 渲染、输入、音频。目标是早期就能在终端里玩到主流游戏，验证产品体验。
- **阶段 2（自实现内核）**：在不改前端的前提下，自实现 6502 CPU、PPU、APU、Bus 与多个 Mapper，完全替换 tetanes，作为项目的"主菜"和学习目标。两阶段通过 `EmulatorBackend` trait 实现可插拔切换。

### 非目标（YAGNI）

- 商业级 ROM 浏览器 / 配置 GUI
- 网络对战 / netplay
- shader / CRT 滤镜
- Mapper 5/9/etc.（除非未来扩展）
- 移动端 / Web 端
- 即时存档（trait 上不预留，未来加）

## 2. 决策记录

| 决策 | 选项 | 理由 |
|------|------|------|
| 实现路线 | **C：混合**——先复用 tetanes，再用自实现替换 | 早期可玩 + 长期学习价值 |
| 语言 | **Rust** | 性能、生态、纯 Rust 路线无 FFI 切换成本 |
| 终端渲染 | **多模式自适应**（halfblock 默认 / braille / ascii） | 兼顾画质、终端宽容度、复古风 |
| 阶段 1 范围 | **MVP-中**（含音频、多渲染、状态栏、暂停/重置） | 平衡完成感与后期预算 |
| 阶段 2 推进 | **测试驱动 + 增量加 mapper**（D） | 用 nestest / blargg 做硬验收，按目标游戏增量解锁 mapper |

## 3. 整体架构

Cargo workspace，硬边界划分内核与前端。

```
glyph8/
├── Cargo.toml                    # workspace
├── crates/
│   ├── nes-core/                 # 抽象层：trait + 数据类型
│   ├── nes-tetanes-backend/      # 阶段 1：tetanes 适配器
│   ├── nes-native/               # 阶段 2：自实现 CPU/PPU/APU/Mapper
│   ├── nes-render/               # 终端渲染
│   ├── nes-audio/                # cpal + 重采样
│   └── nes-input/                # crossterm 键盘 → 控制器
└── glyph8-cli/                   # 二进制
```

`glyph8-cli` 只依赖 `nes-core` + 渲染/音频/输入 + 一个 backend，对内核完全黑盒。

通过 cargo feature 选择 backend：`backend-tetanes` / `backend-native` / `backend-diff`。

## 4. 核心抽象（nes-core）

### 数据类型

```rust
pub struct Frame {
    pub pixels: [u8; 256 * 240],   // NES 调色板索引（0..64），不是 RGB
}

pub const NES_PALETTE: [(u8, u8, u8); 64] = [...];  // NTSC 默认调色板

#[derive(Clone, Copy, Default)]
pub struct ControllerState(pub u8);
impl ControllerState {
    pub const A: u8      = 1 << 0;
    pub const B: u8      = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8  = 1 << 3;
    pub const UP: u8     = 1 << 4;
    pub const DOWN: u8   = 1 << 5;
    pub const LEFT: u8   = 1 << 6;
    pub const RIGHT: u8  = 1 << 7;
}

pub struct RomInfo {
    pub mapper: u8,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub mirroring: Mirroring,
    pub has_battery: bool,
}
```

### Trait

```rust
pub trait EmulatorBackend: Send {
    fn load_rom(&mut self, rom: &[u8]) -> Result<RomInfo, EmulatorError>;
    fn step_frame(&mut self);
    fn frame(&self) -> &Frame;
    fn submit_input(&mut self, p1: ControllerState, p2: ControllerState);
    fn drain_audio(&mut self) -> &[f32];
    fn reset(&mut self);
}

#[derive(thiserror::Error, Debug)]
pub enum EmulatorError {
    #[error("invalid iNES header")]
    InvalidINesHeader,
    #[error("unsupported mapper {0}")]
    UnsupportedMapper(u8),
    #[error("rom too small")]
    RomTooSmall,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

### 设计要点

1. **Frame 用调色板索引**：节省内存（61KB / 帧）；前端查表转 RGB；支持调色板热替换。
2. **音频以 f32 + 源采样率**：backend 不重采样，由 `nes-audio` 处理。
3. **以帧为粒度**：tetanes 直接支持；自实现内部跑约 29780 cycles 凑一帧。
4. **`submit_input` 在帧前调用**：表达"本帧输入快照"，对 CLI 体验完全够。

## 5. 渲染子系统（nes-render）

### Trait

```rust
pub trait Renderer {
    fn capabilities(&self) -> RendererCaps;
    fn render(&mut self, frame: &Frame, palette: &Palette) -> std::io::Result<()>;
    fn handle_resize(&mut self, w: u16, h: u16);
}
```

### 三种实现

| 模式 | 字符 | 像素/字符 | 输出 | 颜色 |
|------|------|-----------|------|------|
| Halfblock | `▀` | 1×2（前景上像素，背景下像素） | 256×120 字符 | TrueColor / 256 |
| Braille | `⠀..⣿` | 2×4（亮度阈值） | 128×60 字符 | 单色 |
| ASCII | ` .:-=+*#%@` | 1×1（按亮度） | 终端最大缩放 | TrueColor / 单色 |

### 渲染管线

```
Frame (256×240 调色板索引)
  → Palette 查 RGB
  → 缩放/裁剪到目标网格
  → Renderer 输出 ANSI 序列到 stdout
```

### 自适应启动

- 检测 `COLORTERM=truecolor` 与终端尺寸
- 256×120 + truecolor → Halfblock
- 不够大但有色 → Halfblock + 缩放
- 仅 256 色 → Halfblock 256
- 都没有 → ASCII

可通过 `--render=halfblock|braille|ascii` 强制覆盖。

### 性能关键点

1. **Diff 渲染**：保留上一帧字符缓冲，只输出变化的字符。NES 60 FPS 下，全量重绘一帧约 10MB ANSI 文本/秒会卡；diff 后通常 5–20% 字符变化。
2. **ANSI 输出缓冲**：单帧用一个 `String` 累积，一次性 `stdout.write_all`。
3. **缩放算法**：最近邻；不做双线性。

### UI 布局（ratatui 仅用于状态栏）

```
┌─ Glyph8 [Super Mario Bros.nes] ────────────────── 60.0 FPS ─┐
│                                                              │
│              [256×120 halfblock 画面]                        │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ P1: ←↑→↓ J=A K=B Enter=Start Space=Select  ESC=quit P=pause  │
└──────────────────────────────────────────────────────────────┘
```

画面区直接 stdout 写 ANSI，不套 ratatui widget；ratatui 仅画边框和状态栏。`?` 键弹出帮助 overlay。

## 6. 输入（nes-input）

### 默认键位

```
方向：W/A/S/D（也接受方向键）
A 键：J
B 键：K
Start：Enter
Select：Space（Shift 在终端不易捕获）
```

### 关键挑战：终端无 keyup 事件

策略：

- **首选**：Kitty Keyboard Protocol（kitty / WezTerm / Ghostty 支持），通过 `crossterm::event::PushKeyboardEnhancementFlags` 启用，能拿到真正的 keyup/keydown。
- **回退**：按键超时窗口（80ms）。每次 keydown 维持"按住"状态 80ms；窗口内无新事件视为松开。状态栏标注 `input: legacy`。

### 线程模型

```
[input thread]                 [main thread]
crossterm EventStream  ──→ channel ──→ current_state
                                          │
                                          ▼
                              backend.submit_input()
                              backend.step_frame()
                              renderer.render()
```

输入跑独立线程，主循环每帧拉一次最新状态。

## 7. 音频（nes-audio）

```rust
pub struct AudioOutput {
    stream: cpal::Stream,
    ring: Arc<Mutex<RingBuffer<f32>>>,
    device_rate: u32,
    source_rate: u32,
}
```

### 要点

1. **重采样**：`rubato::SincFixedIn`；NES APU 源采样率 → 设备采样率（44.1/48kHz）。
2. **背压**：cpal callback 从 ring buffer 取，emulator 推。ring 满则丢最旧。
3. **同步策略**：MVP 用 `std::thread::sleep` 做 60Hz 帧定时（简单先跑通）；后期可切"音频时钟驱动 emulator step"。
4. **`--mute`**：默认开启音频；无设备时 cpal 错误降级到 mute 并提示。

## 8. 阶段 2：自实现内核（nes-native）

### 模块划分

```
nes-native/src/
├── lib.rs                    # 实现 EmulatorBackend
├── cpu/
│   ├── mod.rs                # 状态机
│   ├── opcodes.rs            # 256 个 opcode
│   ├── addressing.rs         # 13 种寻址
│   └── instructions.rs       # 56 官方 + 主要非官方
├── ppu/
│   ├── mod.rs                # scanline / dot 时序
│   ├── registers.rs          # $2000-$2007
│   ├── background.rs
│   ├── sprites.rs            # OAM / sprite 0 hit
│   └── palette.rs
├── apu/
│   ├── mod.rs
│   ├── pulse.rs              # 2 通道方波
│   ├── triangle.rs
│   ├── noise.rs
│   └── dmc.rs                # 后期里程碑
├── bus.rs                    # CPU 总线 + DMA
├── cart.rs                   # iNES 解析
└── mapper/
    ├── mod.rs                # Mapper trait
    ├── nrom.rs               # Mapper 0
    ├── mmc1.rs               # Mapper 1
    ├── uxrom.rs              # Mapper 2
    └── mmc3.rs               # Mapper 4（含 IRQ）
```

### 实现顺序与验收（"测试驱动增量"）

| ID | 内容 | 验收 |
|----|------|------|
| M1 | CPU + Mapper 0 + nestest | `nestest.nes` 自动模式输出与 `nestest.log` 完全一致 |
| M2 | PPU 背景 | 大金刚 / 冰climber 标题画面正确 |
| M3 | PPU 精灵 + sprite 0 hit | blargg `ppu_vbl_nmi.nes`；超级马里奥能玩 |
| M4 | APU（不含 DMC） | blargg `apu_test.nes`；超级马里奥音乐正确 |
| M5 | MMC1 + UxROM | 塞尔达、魂斗罗、洛克人能玩 |
| M6 | MMC3 + DMC | 超级马里奥3、忍者龙剑传能玩 |

### Backend 切换 / Diff 模式

```bash
glyph8 super_mario.nes                    # 默认 tetanes
glyph8 --backend=native super_mario.nes   # 自实现
glyph8 --backend=diff super_mario.nes     # 双跑，每帧 hash 比对，不一致 panic
```

`--backend=diff` 是开发阶段 2 的关键工具：自实现哪一帧偏差，立刻定位。

### 非官方 6502 指令

只实现"主要的"约 60 条（覆盖商业游戏 ~99%）。完全实现没有产出。

## 9. 测试策略

### 测试金字塔

```
        e2e (手动)         几个商业游戏跑通第 1 关
        ──────────
   acceptance ROM           nestest / blargg / sprite_hit
        ──────────
   集成 (headless)          每个 backend × 每个 ROM
        ──────────
   单元测试                  CPU 指令、寻址、mapper、render snapshot
```

### 各层细节

- **单元测试**：每 crate 内 `#[cfg(test)]`。CPU 每指令 ≥ 1 测试；render 用 `insta` snapshot；mapper bank 切换；bus 镜像规则。
- **集成测试**：每个 backend 跑测试 ROM。nestest 与 `nestest.log` 行级比对；blargg ROM 读 RAM 判定 PASSED/FAILED。
- **acceptance ROM**：固定一组 ROM，预录"运行 N 帧后 frame 哈希应为 X"作为 fixture。**fixture 哈希用 tetanes backend 跑出来作为 ground truth，native backend 必须匹配**——这是 C 路线的核心红利。
- **e2e（手动）**：每里程碑结束在真终端跑目标游戏列表，写在 `docs/qa-checklist.md`。

### 测试 ROM

- nestest（CPU）
- instr_test-v5（指令时序）
- ppu_vbl_nmi（VBL/NMI）
- sprite_hit_tests
- scanline
- apu_test
- mmc3_test

均为 nesdev wiki 公开测试 ROM，入仓库或 git submodule。商业 ROM 用户自带，不入仓库。

### 性能基准

`criterion` 跑：单帧 CPU、单帧 PPU、halfblock + diff 输出。

目标：M1 MacBook 上 native backend 单帧 < 8ms。

### 不做的事

- 内核内不 mock：模拟器内部"模拟"已经够多。
- emulator 状态不做快照测试（churn 太高）。
- proptest 仅用于 CPU 寻址。

## 10. 里程碑与时间线

| 阶段 | 里程碑 | 估时（业余） |
|------|--------|------|
| 0 | Workspace + nes-core + iNES 解析 | 0.5 周 |
| 1.A | tetanes backend 接通 | 0.5 周 |
| 1.B | halfblock renderer + 主循环 | 1 周 |
| 1.C | 输入（Kitty + 超时回退） | 1 周 |
| 1.D | 音频（cpal + rubato） | 0.5–1 周 |
| 1.E | braille/ascii + 自适应 | 0.5 周 |
| 1.F | ratatui 状态栏 + 暂停/重置/退出 | 0.5 周 |
| **MVP-中** | | **~5 周** |
| 2.M1 | CPU + Mapper 0 + nestest | 2–3 周 |
| 2.M2 | PPU 背景 | 2 周 |
| 2.M3 | PPU 精灵 | 2 周 |
| 2.M4 | APU | 2 周 |
| 2.M5 | MMC1 + UxROM | 1 周 |
| 2.M6 | MMC3 + DMC | 2 周 |
| **阶段 2 完成** | | **~12 周** |

总体业余约 **4–5 个月**。

## 11. 分发与 CLI

### 发布渠道

1. `cargo install glyph8`（主要）
2. GitHub Releases 预编译二进制：macOS arm64/x64、Linux x64、Windows x64
3. Homebrew tap（个人，可选）

### CLI 用法

```bash
glyph8 super_mario.nes                     # 最简
glyph8 --render=braille zelda.nes          # 切渲染
glyph8 --backend=native contra.nes         # 切 backend
glyph8 --backend=diff --log-cpu super_mario.nes  # 调试
glyph8 --keymap=arrows zelda.nes           # 自定义键位
glyph8 --mute super_mario.nes              # 静音
```

### Cargo features

```toml
[features]
default = ["backend-tetanes"]
backend-tetanes = ["dep:nes-tetanes-backend"]
backend-native = ["dep:nes-native"]
backend-diff = ["backend-tetanes", "backend-native"]
```

### CI 矩阵

GitHub Actions：

- 平台：macOS / Linux / Windows
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- acceptance ROM 测试（含两个 backend）
- `cargo bench`（仅 main 分支，存档对比）

## 12. 文档

- `README.md`：装、跑、键位、终端截图
- `docs/architecture.md`：内核架构 + trait 抽象
- `docs/development.md`：本地开发 + 跑测试 ROM
- `docs/qa-checklist.md`：每个里程碑的手动验收游戏列表
- `docs/superpowers/specs/`：本 spec + 后续每里程碑的 plan

## 13. 风险登记

| 风险 | 缓解 |
|------|------|
| 终端 60 FPS 不稳 | 1.B 阶段先 prototype 渲染管线性能；不行就降 30 FPS 显示 / 60 FPS 模拟 |
| Kitty 协议不普及，超时回退手感差 | README 推荐 kitty/ghostty/wezterm；超时回退视为可接受降级 |
| 自实现 PPU 时序复杂度被低估 | `--backend=diff` 模式快速定位偏差；接受 M2/M3 各延期 1 周 |
| 测试 ROM 版权 | 仅入公开测试 ROM；商业 ROM 用户自带 |
| MMC3 IRQ 时序 | M6 单独里程碑；前 5 个里程碑足以"产品成型" |

## 14. 开放问题（实现阶段决定）

- 即时存档（save state）：trait 不预留；未来若加，作为 trait 扩展或单独 trait `Snapshotable`。
- 2P 手柄：MVP 不做，trait 已支持 P2 输入字段，未来加双键盘布局即可。
- ROM 浏览器 / TUI 选 ROM：YAGNI。
- 调色板切换：`Palette` 已是参数，未来加 `--palette` flag 即可。
