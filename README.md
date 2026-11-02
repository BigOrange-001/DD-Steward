# DD Steward

## 中文

《暗黑地牢1》的独立Rust模拟器与战役策略原型。业务源码同步到029协调修复及030人类操作工具，包含021～028依赖；保留旧入口，不携带私有开发历史。

029协调补给、撤退、恢复与公开推演模型。030提供JSON行操作协议、中文终端、备注、双提示、保存恢复及公开回放影子分析，动作由同一原生会话执行。提示只读公开观察、独立采样，不替用户执行动作。跨步意图为近似；新的56项原生管家清单与历史020目标协议保持独立。

仍是**待原版校准的候选原型**，不用于正式训练。029开发样本完成52/262→67/193，但死亡110→171、全员死亡趟17→37，不能宣称整体改善。030最终验收按用户要求停止；本次公开源码导出未重新构建或测试。

仅提供业务源码、构建文件和本说明；候选业务规则内嵌。没有原版配置/音画资源、冻结场景、回放、私有检查点、内部文档或测试材料。数值与中文名从自行提供的合法安装只读加载，只加载启用内容。单战/场景列表另需提供`data/human-scenes-030.json`及其引用的公开输入，不随源码分发。

## English

An independent Rust simulator and campaign-policy prototype for **Darkest Dungeon 1**. This snapshot adds task029 coherence corrections and task030 human-play tools, with their task021–028 dependencies. Legacy entrypoints remain separate; private development history is excluded.

The human interface offers filtered JSONL observations/actions, a Chinese terminal, notes, two independent advisers, save/resume and offline replay comparisons. Advice never executes a human action. Cross-step intentions are approximate. This remains an **uncalibrated candidate**, without formal training or demonstrated natural completion. Development completion rose from 52/262 to 67/193, while deaths rose from 110 to 171. Final validation was stopped at the user's request; this public export has not been rebuilt or tested.

Only application/build source and this overview are distributed. Candidate business settings are embedded; game data/assets, frozen scenes, replays, private checkpoints, internal documents and test material are excluded. Provide your own legitimate installation and enabled content. Arena mode additionally requires your own public scene catalog and referenced inputs.

## PowerShell

Requires Rust 2024 and Python 3. The terminal wrapper currently targets Windows.

```powershell
$env:DD_GAME_DIR = 'C:\Games\DarkestDungeon'
$env:CARGO_TARGET_DIR = 'target/dev'
cargo build --release -p dd-core --example play030 --example campaign029
New-Item -ItemType Directory -Force target/release | Out-Null
Copy-Item target/dev/release/examples/play030.exe target/release/play030.exe -Force
python scripts/play030.py --start old-road --seed 1 --out runs/human-first
```

Use a new directory per run. Resume and compare:

```powershell
python scripts/play030.py --resume runs/human-first
python scripts/shadow030.py runs/human-first
```

`n text`: note; `h`: advice toggle; `s`: full public view; `l 2`: action page 2; `q`: save/exit. Town parameter menus accept JSON. Destructive actions require `yes`; there is no undo. `steps.jsonl` is public replay data; `checkpoint.json` is private host state. Availability does not establish original-game calibration.
