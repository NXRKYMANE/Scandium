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

| 组件                        | 配置项                  | 行为                                              |
| --------------------------- | ----------------------- | ------------------------------------------------- |
| 服务 (`scandium_svc.exe`) | `eco_qos = "auto"`      | 空闲（CPU < 10%）进入效率模式，繁忙（> 30%）退出  |
| 宿主 (`os.exe`)             | `host_eco_qos = "auto"` | 空闲（CPU < 5%）进入，宿主或服务繁忙（> 20%）退出 |

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

安装包特性：
- 中英文双语界面，默认跟随系统语言
- 智能版本比较：升级静默、同版本询问重装、降级警告
- 安装 `scandium_svc.exe`，写入服务 TOML 配置
- 通过 Osmium 注册并启动服务，全程退出码检查（失败弹「终止 / 重试 / 忽略」）
- 安装前自动等待旧进程退出，避免文件占用（所有安装模式）
- 卸载时通过 Osmium 删除服务并移除所有文件

> **Osmium 集成要点：** TOML 路径含反斜杠需用单引号字面字符串；从注册表 `HKLM\...\App Paths\os.exe` 定位 Osmium；失败时用 `ExecAndCaptureOutput` 捕获退出码并弹「终止 / 重试 / 忽略」。

## 部署

使用 [Releases](https://github.com/NXRKYMANE/Scandium/releases) 的安装包即可完整安装并自动注册服务。

手动部署：
1. 将 `Publish/` 中的 `scandium_svc.exe` 复制到目标机器。
2. 安装 [Osmium](https://github.com/NXRKYMANE/Osmium)（自动注册 `os.exe` 到 PATH）。
3. 注册服务：`os.exe --install scandium_svc.toml`
4. 启动服务：`os.exe --start scandium_svc`

## 免责声明

**本项目本质上只是一个比较鸡肋的工具，无法保证对所有计算机有效。清空进程工作集后，换出的内存页重新读回时可能短暂增加磁盘活动；若无明显效果甚至加重负担，请立即卸载本服务。**

## 赞助

如果这个项目对你有帮助，欢迎[赞助支持](https://ifdian.net/a/NXRKYMANE) 。

## 许可证

本项目基于 Apache License 2.0 开源——详见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。
