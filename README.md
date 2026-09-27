# EldenRingPostFX

使用 Rust 编写的 ModEngine3 Native DLL，按配置移除《艾尔登法环》的色差、暗角和游戏内置锐化。

**0.1.0 为预览版。1.17 的用户对照场景已验证三项效果；1.17.1 已通过静态与资源回归，尚待实机确认。** 实现基准为 App Ver. 1.17 / Regulation 1.17；另对 1.16.2 / 1.16.1 做了离线兼容验证。现有 `0.1.0-preview-2b1f244` 无需因 1.17.1 重新构建，配置方式不变。

作者：Luna(a492219408)。许可：GPL-3.0-only。

## 使用

需要 Windows x64、ModEngine3 和游戏。运行包只需 DLL、INI 与 ME3 配置；无需 Nuxe、WitchyBND、Oodle、Ghidra、DCX 或额外着色器文件。

1. 解压测试包到可写目录，保留目录结构。
2. 首次测试使用包内 `EldenRingPostFX.me3`，通过 ModEngine3 启动游戏。该配置为离线配置，要求 `load_early = true` 及具名 initializer。
3. 进入存档，查看 `natives/EldenRingPostFX.log` 是否出现实际导入统计，再对比画面。看到 `HOOK INSTALLED` 只表示安装挂钩完成。
4. 编辑 `natives/EldenRingPostFX.ini` 后完全退出并重新启动游戏。

```ini
[postfx]
disable_chromatic_aberration = true
disable_vignette = true
disable_sharpening = true
observe_only = false
```

前三个 `true` 分别表示去色差、去暗角、去锐化；`false` 保留游戏或其他 MOD 提供的值。三项可任意组合；全为 `false` 且未启用诊断时不安装挂钩。缺少 INI 时默认三项开启；拼写错误、重复键或无效值会拒绝初始化并记录日志。

`observe_only = true` 安装同一导入挂钩并统计待修改记录，但不修改 GPARAM，可用于诊断。它仍然是运行时挂钩，不等同于完全卸载 DLL。

合并进已有 ME3 配置时，把 `natives` 目录放在自选 MOD 文件夹内，然后添加下面这一段，并按实际位置调整 `path`：

```toml
[[natives]]
path = 'mods/eldenring/EldenRingPostFX/natives/EldenRingPostFX.dll'
enabled = true
optional = false
load_early = true
initializer = { function = "elden_ring_postfx_init" }
```

合并后关闭原资源 MOD 的五种变体：`NoChromaticAberrationNoVignetteNoSharpening`、`NoSharpening`、`NoChromaticAberrationNoVignette`、`NoChromaticAberrationNoSharpening`、`NoChromaticAberration`。旧资源若仍生效，关闭 DLL 中某项不会恢复已经被旧资源置零的值，也无法保留被旧文件覆盖的新版参数。

回退：退出游戏，在 ME3 配置中禁用或移除本 DLL 的 `[[natives]]` 项；需要时再恢复原资源 MOD。DLL 不修改磁盘上的游戏文件。首版不支持热加载、热重载或进程内卸载。

## 日志与首次验证

日志与 DLL 放在同一目录，每次初始化覆盖上一轮日志；需要保留时请在下次启动前复制。

- `INIT FAILED`：配置、目标进程、签名或安装检查失败。ME3 可能继续启动游戏，所以不能仅凭启动成功判定生效。
- `HOOK INSTALLED`：导入挂钩已安装，尚不能证明资源覆盖或画面效果。
- `imports / accepted / rejected / panics`：经过此复制点、通过 GPARAM 解析、被拒绝、被捕获的 Rust panic 次数。
- `rows_found(CA,VIG,SHARP)`：识别到的色差、暗角、锐化记录数。
- `rows_selected(CA,VIG,SHARP)`：所选开关对应的非零记录数；正常模式会置零，诊断模式只计数。重复加载会累计，并非唯一资源数量。
- `last_reject`：1=不是支持的 GPARAM/长度不合；2=版本变化；3=越界；4=布局变化；5=复杂度超限；6=类型变化；7=重叠；100=复制调用的缓冲区条件不符。`first_reject_header_le` 只记录首个被拒绝缓冲区的 8 字节头部。

建议先记录关闭 DLL 时的固定机位，再分别测试三项以及全开；每次更换配置重启游戏。检查室外、室内、赐福休息后的不同时间、传送，以及 DLC 区域。画面验证完成前，不建议仅凭日志宣布某个版本全面兼容。

## 实现范围

游戏为 GPARAM 导入分配新缓冲区后，DLL 将该路径中一处 `CALL` 转接至自己的回调。回调先调用游戏原本的复制函数，再处理复制完成的私有目标缓冲区，保留原始输入、分配器、地址、尺寸、返回值与生命周期。它不全局挂钩 memcpy，不修改渲染指令，也不打包或内嵌游戏资源。

解析器以完整组名和完整字段名匹配，并覆盖字段的全部 ValueID / 时间记录：

| 组 | 字段 | 写入 |
|---|---|---|
| `Tone Map[Yebis]` | `GS Presents - Chromatic Aberration Enable` | 整数 0（类型 3 或 7） |
| `Tone Map[Yebis]` | `GS Presents - Vignetting Power` | 浮点数 0（类型 9） |
| `Anti-Alias-Editor` | `AntiAlias - SharpenIntensity` | 浮点数 0（类型 9） |

验证 GPARAM v5 的相关边界、类型、值区间和 ID 区间后才写入；被拒绝的缓冲区保留原值。其他字段、时间、记录长度及注释保持不变。只在资源导入时处理，没有逐帧内存扫描。

PE 定位使用唯一指令模式、异常表中的函数边界、分配路径、LUT 字符串、三个字段名和复制函数特征共同校验。调用点通过附近的只读可执行跳转页抵达 Rust 回调。代码写入完成后恢复页权限并刷新指令缓存，DLL 和跳转页保留至进程结束。

这些校验允许符合相同结构的后续版本尝试加载；任何结构变化仍可能导致拒绝或失效，不保证未来版本兼容。资源格式、入口覆盖、DLC 实景、与其他参数或渲染 MOD 的组合仍需要实机验证。该项目移除的是原 GPARAM MOD 对应的游戏效果，外部 ReShade、显卡驱动和其他滤镜的锐化不在范围内。

## 已完成验证

- 两版各 448 份本地官方 GPARAM，8 种组合，共 7,168 次与独立参考变换的完整字节比较通过。
- 1.16.2 / 1.16.1 中原 MOD 的 1,801 份解压文件全部复现；1.17 中四个变体的 `m61_00_0000` 保留新版非目标参数，未用旧资源覆盖。
- 1.17.1 的 448 份官方绘图参数与 1.17 完整字节相同；现有程序对 8 种组合的 3,584 次完整字节回归比较全部通过。
- 1.16.2、1.17、1.17.1 的 EXE 均能唯一定位导入复制点；分配指令被改动或签名重复时拒绝定位。1.17.1 的相关函数结构与已验证的 1.17 一致。
- 1.17 的一个用户测试场景已提供三项效果的截图证据及全开运行日志。范围限于该场景；1.17.1 尚未实机验证。
- 人工构造数据验证开关独立性、所有时间行、重复应用、观察模式、错误类型、重叠、截断及失败不产生部分修改。
- Windows 测试进程实际执行相对 CALL、附近跳转页和回调，验证返回及原始输入保持正确。**这些不是游戏实测。**

## 从源码构建

需要 Rust 1.96 或更新版本、`x86_64-pc-windows-msvc` 目标和 Visual Studio C++ 链接工具。没有第三方 Rust crate 依赖，不依赖私有仓库、游戏样本或维护者路径。

```powershell
mise exec -- cargo fmt --all -- --check
mise exec -- cargo clippy --all-targets -- -D warnings
mise exec -- cargo test --all-targets
mise exec -- cargo build --release --target x86_64-pc-windows-msvc
```

没有使用 mise 的环境可直接调用 `cargo`。运行 `scripts/Build.ps1` 会执行检查、构建并生成 `dist` 下的运行 ZIP 和源码 ZIP；源码 ZIP 来自当前干净的 Git 提交。脚本不安装 MOD、不修改现用 Profile、不启动游戏。

`postfx-audit` 是可选离线验证工具，不是运行时依赖：

```text
postfx-audit pe <eldenring.exe>
postfx-audit patch <0..7 mask> <raw.gparam> <new-output.gparam>
postfx-audit patch-dir <0..7 mask> <input-directory> <new-output-directory>
```

掩码位：色差=1、暗角=2、锐化=4。只读取自备的解压 GPARAM；输出必须是新文件/新目录。默认测试只使用人工样本；可选 EXE 测试通过 `POSTFX_TEST_EXE` 提供自己的样本，再执行 `cargo test --release --test local_pe -- --ignored --nocapture`。所有真实游戏数据都不进入源码、运行包或默认测试。

研究来源与致谢见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)，完整许可见 [LICENSE](LICENSE)。
