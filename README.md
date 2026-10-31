# DD Steward

## 中文

《暗黑地牢 1》的独立 Rust 规则模拟器与 AI 战役原型。已接入候选战斗、探索、城镇成长、跨周战役、庭院与农庄，以及仅依赖玩家可见观察的自动策略、文字监控和保存恢复。

当前管家目标共 **90 项：58 项 Boss、13 项主要内容、19 项职业培养**（含当前配置的扩展职业）。普通英雄必须在实际副本返程时升到六级才完成职业目标；星晶佣兵不计，成就达成后保留。

目前仍是**待原版校准的候选原型**，采用启发式策略，尚未证明自然完成全部目标，也未进入正式 AI 训练。此仓库仅提供业务源码与构建文件，不含游戏数据、资源、存档、内部文档或测试记录。候选规则随源码保留；原版数值仍从外部游戏配置读取。

需 Rust 2024 工具链，并自行提供合法游戏安装及所启用的 DLC/扩展数据。设置 `DD_GAME_DIR` 后运行：

## English

An independent Rust rules simulator and AI campaign prototype for **Darkest Dungeon 1**. It includes candidate combat, exploration, town progression, campaign weeks, Courtyard and Farmstead flows, plus a heuristic policy using only player-visible observations, text monitoring, and save/resume.

The current Steward contract tracks **90 objectives: 58 boss targets, 13 major-content milestones, and 19 class-growth achievements**, including configured expansion classes. A regular hero must reach level six during an actual expedition return; shard mercenaries do not qualify, and earned achievements persist.

This is an **uncalibrated candidate prototype**. Natural completion of all objectives has not been demonstrated; formal AI training has not started. Only application source and build files are provided—no game data/assets, player saves, internal documents, or test records. Candidate rules are embedded in source; original game values are loaded from external game files.

Requires a Rust 2024 toolchain and your own legitimate installation with the enabled DLC/expansion content. Set `DD_GAME_DIR`, then run:

```powershell
$env:DD_GAME_DIR = 'C:\Games\DarkestDungeon'
cargo build --release --workspace
cargo run --release -p dd-core -- auto020 --candidate --seed 1 --steps 500 --out runs/demo
```

`--steps` limits this run's prefix; it does not mean victory. Resume with / 续跑：

```powershell
cargo run --release -p dd-core -- auto020 --candidate --resume runs/demo/checkpoint.json --steps 500
```
