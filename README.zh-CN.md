# Elden Ring Native PostFX

[English](README.md)

Rust 编写的 **ModEngine3 Native DLL**，独立控制去色差、去暗角和去游戏内置锐化。无需替换游戏资源，也不需要额外着色器。

**1.0.0 正式版 · Windows x64 · GPL-3.0-only · Luna(a492219408)**

正式版保留 `0.1.0-preview-2b1f244` 的效果处理与挂钩逻辑，完善版本标识、打包、文档和许可。维护者已在 **App Ver. 1.17.1 / Regulation Ver. 1.17.1** 运行预览版，暂未发现问题；提供的日志显示三项修改均有实际处理记录，没有解析拒绝或捕获到的 panic。这不代表全地图或所有 MOD 组合已验证。

## 安装与配置

需要 Windows x64、游戏和支持提前加载及具名初始化函数的 [ModEngine3](https://github.com/garyttierney/me3)。

1. 将 `EldenRingPostFX-1.0.0.zip` 解压到可写目录，保留文件结构。
2. 如安装过原 **No Chromatic Aberration no Vignette no Sharpening** 资源 MOD，关闭其全部五种变体。
3. 用 ModEngine3 打开包内 `EldenRingPostFX.me3` 并启动游戏。示例配置使用离线模式。
4. 默认移除三项效果。修改 `natives/EldenRingPostFX.ini` 后，完全退出并重新启动游戏。

```ini
[postfx]
disable_chromatic_aberration = true
disable_vignette = true
disable_sharpening = true
observe_only = false
```

前三项依次为去色差、去暗角和去锐化。`true` 移除对应效果；`false` 保留读入值，不能恢复已被其他 MOD 移除的效果。三项任意组合；全部为 false 且未启用观察模式时，不安装挂钩。缺失 INI 时使用默认值；未知键、重复键或无效值会拒绝初始化。

`observe_only = true` 为诊断模式：统计将修改的非零记录，不写入 GPARAM，但仍安装导入挂钩。

无需 Nuxe、WitchyBND、Oodle 或 Ghidra；它们只用于开发研究。CRT 静态链接，无需额外安装 Visual C++ 运行库。

## 合并到已有 ME3 配置

DLL 与 INI 放在同一目录。在自己的 `.me3` 文件中加入以下条目，按相对于该配置文件的位置调整 `path`：

```toml
[[natives]]
path = 'mods/eldenring/EldenRingPostFX/natives/EldenRingPostFX.dll'
enabled = true
optional = false
load_early = true
initializer = { function = "elden_ring_postfx_init" }
```

必须保留 `load_early` 和具名 initializer。关闭原资源 MOD 的全部五种变体，包括单独去色差和单独去锐化。否则旧资源仍可能覆盖新版游戏参数，DLL 中关闭某项也无法恢复旧资源已置零的值。

卸载：退出游戏，禁用或删除本 DLL 的 native 条目。程序不改写磁盘上的游戏文件，不支持热加载、游戏内重载配置或进程内卸载。

## 兼容范围

| 游戏 / 规则版本 | 证据 |
| --- | --- |
| 1.17.1 / 1.17.1 | 用户对预览版的运行反馈与日志；EXE 和资源回归检查 |
| 1.17 / 1.17 | 用户固定机位的三项效果截图；EXE 和资源检查 |
| 1.16.2 / 1.16.1 | 离线 EXE 和资源检查，尚无 DLL 实机验证 |
| 后续版本 | 结构保持兼容时可继续定位，不承诺未知版本可用 |

分析样本来自有 DLC 版。全地图、全部天气与时间、DLC 区域、无 DLC 安装和各类 MOD 组合尚未全面实测。展示用截图来自 1.17，不是 1.17.1。不作性能提升承诺。

只处理原 GPARAM MOD 对应的三项游戏效果，不处理 ReShade、显卡驱动或其他滤镜提供的锐化。与参数 MOD 并用时，选中的目标字段置零，其他字段保留读入值；修改相同导入函数的 DLL 可能冲突。

## 实现概要

游戏为 GPARAM 导入分配并复制新缓冲区时，DLL 转接一处经验证的复制调用：先执行原复制函数，再在游戏新分配的目标副本中处理参数。源数据、分配器、地址、尺寸、返回值和生命周期保持不变；没有全局 memcpy 挂钩或逐帧扫描。

| 组 | 完整字段名 | 写入 |
| --- | --- | --- |
| `Tone Map[Yebis]` | `GS Presents - Chromatic Aberration Enable` | 整数 0，类型 3 或 7 |
| `Tone Map[Yebis]` | `GS Presents - Vignetting Power` | 浮点 0，类型 9 |
| `Anti-Alias-Editor` | `AntiAlias - SharpenIntensity` | 浮点 0，类型 9 |

覆盖字段的全部 ValueID / 时间行。GPARAM v5 的边界、类型和区间重叠校验通过后才写入；被拒绝的缓冲区不变。EXE 定位联合检查唯一指令模式、函数边界、分配路径、字符串与复制函数结构，不采用固定地址或游戏版本哈希白名单；结构校验不能保证未知更新一定兼容。

旧完整资源包的 `m61_00_0000` 较旧。本 DLL 保留新版游戏在此文件中的非目标参数更新，避免旧资源覆盖。

## 日志

日志在 `natives/EldenRingPostFX.log`，每次初始化覆盖。报告问题前先复制日志，并提供游戏/规则版本、INI 和启用的其他 MOD。

- `INIT FAILED`：配置、目标进程、定位或安装检查失败。游戏启动成功不等于 MOD 生效。
- `HOOK INSTALLED`：挂钩安装完成；进入存档后才有实际导入统计。
- `imports / accepted / rejected / panics`：经过的调用、通过解析、被拒绝、捕获的 Rust panic 次数。
- `rows_found(CA,VIG,SHARP)`：识别的三类记录；`rows_selected`：选中且非零的记录，正常模式置零，观察模式只计数。重复加载累计，不等于唯一资源数。
- `last_reject`：1=头部/长度，2=版本，3=越界，4=布局，5=复杂度，6=类型，7=重叠，100=复制缓冲条件；`first_reject_header_le` 只记录首个拒绝缓冲区的八字节头部。

## 构建与打包

Windows 环境需要 Rust 1.96 或更新版本、`x86_64-pc-windows-msvc` 目标和 Visual Studio C++ 构建工具。无第三方 Rust crate，不依赖私有仓库或游戏样本。

```powershell
mise exec -- pwsh -NoProfile -File scripts/Build.ps1
```

没有 mise 时直接用 PowerShell 执行脚本。脚本进行格式、Clippy、自动测试和 release 构建。DLL 位于 `target/x86_64-pc-windows-msvc/release/elden_ring_postfx.dll`。源码 ZIP 解压后不需要 Git 或私有目录即可构建。

打包另需干净的 Git 工作区、Git、PowerShell 7 和 Python 3.11+：

```powershell
mise exec -- pwsh -NoProfile -File scripts/Package.ps1
```

打包会重新构建，检查公开文件白名单与全部可达历史、DLL 导入/导出和 ME3 配置，再生成运行包、对应源码包及 SHA-256 文件。已有制品不会覆盖。`BUILD_INFO.json` 记录源码提交和编译器。更换 Rust 工具链后分发时，应同步更新 `third_party/Rust-COPYRIGHT-library.html`。

默认测试只用人工样本，包含 Windows CALL/跳转页/回调执行测试。可选离线工具 `postfx-audit` 支持 `pe`、`patch` 和 `patch-dir`；掩码为色差=1、暗角=2、锐化=4，只读自备样本，输出必须为新路径。设置 `POSTFX_TEST_EXE` 后可运行 `cargo test --test local_pe -- --ignored --nocapture` 做 EXE 定位及突变拒绝测试。游戏样本不随源码发布。

许可与致谢见 [LICENSE](LICENSE)、[NOTICE](NOTICE) 和 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。分发二进制时应按许可提供与其对应的完整源码。
