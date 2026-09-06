# 来源、许可与致谢

EldenRingPostFX 的原创 Rust 实现由 **Luna(a492219408)** 编写，Copyright (C) 2026，采用 **GPL-3.0-only**。本声明不对游戏资源或第三方作品重新授权。

- **SoulsFormatsNEXT / SoulsFormats**：GPARAM 的公开格式描述与实现为组表、字段表、类型编号和 ValueID/时间布局提供参考。格式实现历史中包括 YabberAvocado、SoulsFormats 和 Natsu 的贡献。本项目重新编写边界检查及变换逻辑，没有链接、嵌入或分发该库。参考：[GPARAM.cs](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/SoulsFormats/Formats/GPARAM.cs)、[上游 GPL 许可](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/LICENSE)。
- **iogus / ioguss**：原 [No Chromatic Aberration no Vignette no Sharpening](https://www.nexusmods.com/eldenring/mods/8570) 是功能对照来源。使用维护者已有本地副本做离线对照，没有复制、内嵌或分发其资源文件，也不声称获得其背书或资产再发布授权。
- **ModEngine3 贡献者**：提供 Native DLL 加载机制。ME3 是单独安装的运行环境，本包不包含其程序或源码。参考：[ModEngine3](https://github.com/garyttierney/me3)。
- **Microsoft**：Windows 内存及 DLL API 按官方文档使用；参考 [VirtualAlloc](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc)、[VirtualProtect](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualprotect)、[DLL 最佳实践](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices)。

运行时只使用 Rust 标准库及 Windows 系统 API，没有第三方 Rust crate。分析阶段使用的 Nuxe、WitchyBND、Oodle 和 Ghidra 不包含在产品中，也不是构建或运行依赖。代码中的短指令特征用于独立定位 GPARAM 导入结构，不含完整游戏函数、着色器或资源。

分发二进制时应随附完整 GPL-3.0 文本，并按 GPL 要求提供与其对应的完整源码。构建脚本会同时产生当前 Git 提交的源码包。
