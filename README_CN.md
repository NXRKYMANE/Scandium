# ✨ Scandium — Windows RAM Clean Service

<p align="center">
  <img src="https://img.shields.io/github/followers/NXRKYMANE?style=social" />
  <img src="https://img.shields.io/github/forks/NXRKYMANE/Scandium" />
  <img src="https://img.shields.io/github/stars/NXRKYMANE/Scandium" />
  <img src="https://img.shields.io/badge/-Rust-FFFFFF?style=flat&logo=rust&logoColor=black" />
  <img src="https://img.shields.io/badge/Gitee-NXRKYMANE-FFFFFF?style=flat" />
  <img src="https://img.shields.io/badge/AtomGit-NXRKYMANEX-FFFFFF?style=flat" />
  <img src="https://img.shields.io/badge/Douyin-Ozones-FFFFFF?style=flat&logo=tiktok&logoColor=white" />
  <img src="https://vbr.nathanchung.dev/badge?page_id=NXRKYMANE.Scandium&color=FFFFFF&leftColor=555555&label=Views" />
</p>

一个轻量高性能的物理内存清理服务，完全基于原生 Win32 API 构建。 [SEE ENGLISH DOCS](README.md)

采用高性能 **Rust** 编写，编译为单个原生二进制（约 116 KB，UPX 压缩），目标机器无需任何运行时与外部引擎。

> 本项目在进程内原生完成内存清理：使用 Toolhelp32 API 枚举系统进程，通过 `SetProcessWorkingSetSize(-1, -1)`（EmptyWorkingSet）清空其工作集，再用 `NtSetSystemInformation` 清空系统 Standby 缓存——全部集成在单个自包含二进制中。

## 工作原理

1. 每 60 秒一个周期，四个轻量引擎按实时 CPU / 内存 / 磁盘采样各自定档交错执行：
   - **工作集引擎**（内核级 `MemoryEmptyWorkingSets`，失败回退逐进程 EmptyWorkingSet）：内存 <50% 保底 1 次/分，按 50/70/85/95% 分档升至 5 次/分，日志按 `Used` 格式
   - **Standby 引擎**（脏页写回磁盘 + 清 Standby / 低优先级 Standby）：1 次/分，内存 ≥80% 提速到 2 次/分；磁盘 ≥60% 忙时跳过脏页写回，日志按 `Standby` 格式
   - **文件缓存引擎**（系统文件缓存）：1 次/分，仅内存 ≥50% 时执行——内存充足时清理收益低于文件性能损失
   - **维护引擎**（注册表缓存整理 + 合并物理内存列表）：固定 1 次/分
2. **资源感知门控：** CPU ≥85% 时全部引擎本周期暂停；工作集引擎在 CPU ≥30% / ≥60% 时再逐级降档，避免在老旧机器上造成清理引发的负载尖峰。
3. 每次工作集清理清空全部进程工作集（临时换出不活跃内存页），对比清理前后内存，任务均匀分布在整个周期内。
4. 缓存引擎通过提升 `SeProfileSingleProcessPrivilege` 与 `SeIncreaseQuotaPrivilege` 特权清空系统缓存列表（服务 LocalSystem 账户与手动管理员运行均可用）。
5. 单实例互斥锁防止并发清理互相冲突。

## 效率模式（EcoQoS）

服务进程与 Osmium 宿主均运行在任务管理器"效率模式"（ProcessPowerThrottling）下，按 CPU 负载自动开关：

| 组件                      | 配置项                  | 行为                                              |
| ------------------------- | ----------------------- | ------------------------------------------------- |
| 服务 (`scandium_svc.exe`) | `eco_qos = "auto"`      | 空闲（CPU < 10%）进入效率模式，繁忙（> 30%）退出  |
| 宿主 (`os.exe`)           | `host_eco_qos = "auto"` | 空闲（CPU < 5%）进入，宿主或服务繁忙（> 20%）退出 |

调整阈值：修改已部署配置 `ProgramData\Osmium\svcs\scandium_svc.osiml`（`eco_qos_idle_cpu_pct` / `eco_qos_busy_cpu_pct` / `host_eco_qos_*` 字段），然后执行 `os.exe --refresh scandium_svc`。

## 项目结构

```
Scandium/
├── Project/                         # Rust 服务源码与构建（主实现）
│   ├── service_core.rs              # 主程序（四引擎调度 + 内存监控 + 清理逻辑）
│   ├── main.rs                      # 程序入口
│   ├── build.rs                     # 构建脚本（版本资源 + 嵌入 UAC 清单）
│   ├── Cargo.toml                   # 项目文件（edition 2024 / release 极致优化）
│   └── installer.iss                # Inno Setup 安装脚本
├── Misc/                            # 资源文件
│   ├── Background.bmp / .png        # 安装向导左侧背景图（源图 + 位图）
│   ├── Proj.bmp                     # 安装向导右上角小图（由 Proj.png 生成）
│   ├── Proj.ico                     # 安装包与程序图标
│   └── Proj.png                     # 图标源图
├── Publish/                         # 构建产物（发布 exe 与安装包）
├── .github/                         # GitHub 社区模板（Issue / PR）
├── app.manifest                     # UAC 管理员清单 + DPI 感知
├── .release.ps1                     # 一键构建脚本（编译 → 发布 → 打包）
├── .gitattributes                   # Git 语言统计排除（安装脚本 / 周边脚本）
├── CLAUDE.md                        # AI 助手规则
├── CHANGELOG.md                     # 开发记录/版本历史
├── CODE_OF_CONDUCT.md               # 行为准则
├── CONTRIBUTING.md                  # 贡献指南
├── LICENSE                          # 许可证（Apache-2.0）
├── NOTICE                           # 版权归属与第三方组件声明
├── README.md                        # 英文文档
├── README_CN.md                     # 中文文档
└── SECURITY.md                      # 安全政策
```

## 运行要求

**运行：**
- Windows 10 / 11（或等效的 Windows Server）
- 管理员权限（已包含 UAC 清单）
- [Osmium](https://github.com/NXRKYMANE/Osmium) — 前置框架，用于将 Scandium 注册为 Windows 系统服务（建议 v26.12.1 或更新版本）

**构建：**
- [Rust](https://www.rust-lang.org/tools/install)（stable，edition 2024）
- Inno Setup 7（仅打包时需要）

## 构建

在项目根目录执行：

```bash
.\.release.ps1
```

发布产物为单个原生可执行文件：`scandium_svc.exe`（Rust，无运行时依赖）。

## Inno Setup 安装包

构建安装包：

```bash
# 1. 先构建项目（如上）
# 2. 安装 Inno Setup（https://jrsoftware.org/isdl.php）
# 3. 编译安装包
ISCC.exe Project\installer.iss
```

输出：`Publish\scandium-svc-win-x64-setup.exe`。

## 部署

使用 [Releases](https://github.com/NXRKYMANE/Scandium/releases) 的安装包即可完整安装并自动注册服务。

手动部署：
1. 将 `Publish/` 中的 `scandium_svc.exe` 复制到目标机器。
2. 安装 [Osmium](https://github.com/NXRKYMANE/Osmium)（自动注册 `os.exe` 到 PATH）。
3. 注册服务：`os.exe --install scandium_svc.toml`
4. 启动服务：`os.exe --start scandium_svc`

## 免责声明

> [!WARNING]
> **本项目可能在部分老旧硬件上产生较高的脉冲式资源占用。请确保电脑使用 DDR4/DDR5 内存与 SSD/NVMe 磁盘；若服务运行期间感到明显卡顿甚至卡死，请尽快卸载本服务。**

## 开发历史

> [!NOTE]
> 项目取名 **Scandium（钪）**，是希望项目能让电脑内存更"抗造"——在我的三台电脑上实测都稳定且有效。
>
> 同时缩写 **Scan** 也有"扫描"之意，与扫描内存并清理的功能相呼应。

> 记得初一那年（大约 2022 年，GPT-3 刚诞生的时代），我对 Python 非常感兴趣，父母便给我报了一个 Python 线上课程。当时我用的是一台用了三四年的老电脑，只有 8GB 内存，想流畅玩 Minecraft Java 版都经常闪退。
>
> 起初我没钱买 MC 正版账号，了解到一个叫 HMCL 的启动器可以玩 MC，但下载模组非常慢，后来换成了 PCL2 启动器——偶然发现 PCL2 的内存清理功能非常好用：基本上运行一次内存占用能降一半，不过过几分钟又弹了回来。加上我有还算不错的 Python 基础，便写了个脚本让 PCL2 每分钟按固定次数运行清理。
>
> 后来为了实现自动化并便于分发，踩了非常多的坑。我最初只想让它开机自启，把快捷方式放进开始菜单的启动文件夹，却总是弹出"请以管理员身份运行"的问题；用 UAC 重新打包后依然没能启动。
>
> 这让我非常恼火。于是继续查资料，了解到了"系统服务"这个机制，想着把 Python 程序写成 Win32 系统服务应该就能绕开这个问题——结果发现 PyInstaller 打包后服务无法正常运行。翻了大量资料才知道是 PyInstaller 会漏掉 `win32timezone` 这个库，手动补进去后还是死活跑不起来，实在没办法，大概是 Python 调用系统 API 太折腾了。
>
> 于是我开始琢磨有没有什么东西能绕过 Python 的固有限制。说来也快，我很快就发现了 GitHub 上的 WinSW 项目，并用它成功把我的 exe 封装成了系统服务——项目雏形就此诞生，我甚至创立了一个工作室，并给项目取名 WRCS（Windows RAM Clean Service）。
>
> 不过新的问题接踵而至：当时用的安装包生成工具是 Advanced Installer，每次通过 WinSW 注册、启动、卸载服务都容易出问题，管理员权限的处理也很繁琐，甚至出现过编译出的安装包在我电脑上正常运行、换一台电脑就报错的诡异情况。
>
> 这再次让我陷入了迷茫。不过那时我已经初三了，为了应付中考只能暂时放弃。
>
> 为了践行"原生、高性能、便于分发、轻量"的宗旨，2025 年暑假我学习了 C# 与 Rust 的基础，从此可以无缝调用系统 API 与 DLL，并在 AI 的辅助下写出了第一个真正能用的项目，取名 Hydride（氢化物），寓意让电脑更"轻"。
>
> 到了今年，我对 WinSW 进行了深度改造，做出了它的超集项目 Osmium（锇；原名 Silanes 硅烷，更早之前则是用 Python 套 WinSW 的胶水项目 WSF——Windows Service Framework），功能非常强大且稳定。
>
> Hydride 项目也随之深度改造为原生对接 Osmium：脱离 PCL2，实现了等效且智能的原生自动化服务，并更名为如今的 Scandium（钪）。安装包改用 Inno Setup 编译，体积更小、可扩展性也更高。

## 赞助

如果这个项目对你有帮助，欢迎[赞助支持](https://ifdian.net/a/NXRKYMANE) 。

## 许可证

本项目基于 Apache License 2.0 开源——详见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。
