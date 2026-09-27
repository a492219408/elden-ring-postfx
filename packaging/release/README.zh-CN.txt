Elden Ring Native PostFX 1.0.0
作者：Luna(a492219408) | 许可：GPL-3.0-only

运行要求
Windows x64、艾尔登法环和支持提前加载及具名 initializer 的 ModEngine3。
ModEngine3：https://github.com/garyttierney/me3
无需替换资源、额外着色器或额外 Visual C++ 运行库。

安装
1. 完整解压到可写目录，保留文件结构。
2. 如安装过原 No Chromatic Aberration no Vignette no Sharpening 资源 MOD，
   关闭全部五种变体，包括仅去色差和仅去锐化。
3. 用 ModEngine3 打开 EldenRingPostFX.me3 并启动。此示例使用离线模式。
4. 默认去除三项效果。编辑 natives/EldenRingPostFX.ini，完全退出并重启游戏。

三个开关依次是去色差、去暗角、去锐化。true 移除，false 保留读入值。
observe_only 通常保持 false；true 只统计，不修改参数。
关闭某项不能恢复已被其他 MOD 改掉的参数。本 MOD 不处理 ReShade 或驱动滤镜。

合并已有配置
DLL 和 INI 放在一起；在自己的 ME3 配置中加入以下条目。
path 按相对于该配置文件的位置调整，保留 load_early 和 initializer：

[[natives]]
path = 'mods/eldenring/EldenRingPostFX/natives/EldenRingPostFX.dll'
enabled = true
optional = false
load_early = true
initializer = { function = "elden_ring_postfx_init" }

卸载
退出游戏，禁用或删除本 native 条目。磁盘游戏文件没有修改。
不支持热加载、游戏内重载配置或进程内卸载。

排查
日志在 natives/EldenRingPostFX.log，每次初始化覆盖。
进入存档后检查实际 imports 统计；HOOK INSTALLED 只代表安装成功。
初始化失败时检查 INI、提前加载设置和 initializer 拼写。
反馈时先保存日志，并提供游戏/规则版本、INI 和启用的其他 MOD。

验证范围
相同效果与挂钩逻辑的 0.1.0 预览版已由维护者在 1.17 和 1.17.1 使用。
正式版重新构建，详细范围见 RELEASE_NOTES.txt。后续版本、所有区域与 MOD
组合不保证兼容；不承诺性能提升。

源码与许可
BUILD_INFO.json 记录准确源码提交；对应源码包为
elden-ring-postfx-1.0.0-source.zip。安装使用不需要下载源码。
再分发时保留完整许可、NOTICE、第三方和 Rust 版权声明，并按许可提供对应源码。
