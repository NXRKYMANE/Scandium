# 项目规则

## 注释
- 注释块不超过两行；单行注释过长时折叠为两行

## 代码质量（每次编辑后检查）
- 优化冗余代码，消除死代码
- 优先合并可合并的代码，合并后复查是否还能进一步删除
- 清理未使用的 use 导入（Rust）
- Rust 代码一律使用 rustfmt 默认规则格式化（`cargo fmt --manifest-path Project\Cargo.toml`），提交前运行 `cargo fmt -- --check` 确认无 diff
- 编译验证：`cargo clippy --release --manifest-path Project\Cargo.toml` 必须 0 警告 0 错误

代码部分始终注释使用中文记录，避免使用英文注释
每次对话完,整理claudemd文件,同时如果项目代码部分有重要变动请在claudemd中记录下来
每次推送代码到仓库时消息总结一个并且使用英文,建立新版本release资产时要和往期版本的格式一致,并且也使用英文

## 项目结构
- Project/：Rust 服务源码（main.rs / service_core.rs / build.rs / installer.iss / Cargo.toml）
- Misc/：图标与安装向导图片
- Publish/：构建产物（已被 gitignore）
- README.md（英文）+ README_CN.md（中文），首行互为语言切换链接

## 重要变动记录
- 2026-08-28：二进制更名 scandium_svc64 → scandium_svc——Cargo.toml [[bin]] 名、build.rs FileDescription、BUILD.ps1 复制/UPX 目标、installer.iss 全部引用（exe 名/TOML 文件名/service_name/Osmium 参数/App Paths）同步；升级兼容：PrepareToInstall 静默清理旧名（--stop/--delete scandium_svc64 + 删旧 App Paths 键），[InstallDelete] 删除旧 exe/TOML 遗留文件，taskkill/tasklist 改 scandium_svc*.exe 通配符同时覆盖新旧名；README 双语/CONTRIBUTING/bug_report 日志路径同步
- 2026-08-28：调度错峰与低占用降档——四引擎首触发错开（WS 0s / Standby +10s / FileCache +20s / Maint +30s，Engine::new 加 offset_ms），消除周期起点三引擎齐发导致的 CPU 71~75% 尖峰与 EcoQoS 反复横跳；工作集分档由 25% 线性档改为 50/70/85/95% 阈值档（<50% 保底 1 次/分，锯齿现象下低占用无净收益）；Used 行去掉百分比仅保留 MB 差值（mem_delta_str 加 with_pct 参数，百分比信息归 Cycle 汇总行专属）
- 2026-08-28：采样修正与周期汇总——CPU 公式修正（GetSystemTimes 的 kernel 时间已含 idle，总量改 kernel+user，修复空闲机恒显 ~51%）；磁盘计数器由 % Active Time 改为 % Disk Time（新版 Win11 无 Active Time 计数器导致恒 0），初始化失败改记 WARN；新增每周期末汇总行 `Cycle: Mem a% → b% (ΔMB)` 作为总体参照（Used/Standby 行仍为各引擎分项，与总体降幅不再混淆），Used 行格式化抽取 mem_delta_str 复用
- 2026-08-28：仓库历史重建——双远端（Gitea origin + GitHub）均强推为单根提交「Move to personal Gitea」（消息取自 Osmium 项目 f914d47），本地 tag v3.0.0/v4.0.0 删除；随后 reflog 过期 + git gc --prune=now --aggressive 本地瘦身，.git 由 3.05 MB（旧 LIBPCL2.dll blob + 多次强推残留松散对象）降至 0.20 MB，fsck 校验通过
- 2026-08-28：移除 Release 镜像同步——删除 .github/workflows/release-sync.yml 与 .github/scripts/sync-releases.ps1（Gitee/AtomGit 镜像不再维护，2026-08-21 引入），README 双语结构树 .github/ 描述回退为仅社区模板、EcoQoS 表格按显示宽度（中文 2 列）重新对齐
- 2026-08-28：接入 app.manifest——仓库根目录 app.manifest（requireAdministrator + PerMonitorV2）此前未被 build.rs 引用（README 声称已含清单与实际不符），build.rs 新增读取嵌入（set_manifest + cargo:rerun-if-changed=../app.manifest，根文件为唯一来源）；手动双击 exe 自动弹 UAC 提权，SCM 服务启动不受 UAC 清单影响；已验证发布 exe 二进制内含 requireAdministrator
- 2026-08-28：双引擎拆分为四轻量引擎——工作集 / Standby 链（脏页写回+清 Standby+清低优先级）/ 文件缓存 / 维护（注册表整理+合并内存），Engine 结构体统一调度周期内交错执行；新增 PDH 磁盘活动采样（PhysicalDisk(_Total)\% Active Time，#[link(name="pdh")]，基线差值，失败退化为 0 不拦截）；磁盘 ≥60% 忙跳过脏页写回（保护机械盘）、Standby 链内存 ≥80% 提速 2 次/分、文件缓存仅内存 ≥50% 清理、CPU ≥85% 全停；大周期仍 60s；README 双语同步
- 2026-08-28：修复管理员手动运行时 SystemFileCache ACCESS_DENIED——NtSetSystemInformation(21) 需 SeIncreaseQuotaPrivilege(LUID=5) 处于启用态（管理员令牌默认存在但禁用，LocalSystem 默认全启用）；新增 SE_INCREASE_QUOTA 常量，enable_standby_privilege 改为 enable_privileges 循环启用 13+5 两特权；不再局限于 LocalSystem 账户，README 双语特权描述同步
- 2026-08-28：依赖版本显式固定——Cargo.toml 中 ctrlc = "3.5.2"、chrono = "0.4.45"（原 "3.4"/"0.4" 浮动范围）
- 2026-08-28：Osmium 更新核查（v26.12.1，2026-08-27 发布）——全集成点（--install/--start/--stop/--delete、TOML 字段含 eco_qos、App Paths\os.exe、ProgramData\Osmium\svcs、--refresh）无破坏性变更；两处修复直接影响本项目（单参数 --install panic 修复、服务更新挂起丢服务修复），README 双语「运行要求」标注建议 Osmium ≥ v26.12.1；修正 README 双语 NSIS 残留表述「静默模式 /S 等待旧进程」→「安装前自动等待旧进程退出（所有安装模式）」
- 2026-08-28：缓存引擎补全脏页写回——新增 MEMORY_FLUSH_MODIFIED_LIST(3) 命令，置于 PurgeStandby 之前（脏页写回磁盘后落入 Standby 才能被清除）；README 双语清理链描述同步
- 2026-08-28：健壮性与重构修复——get_standby_mb 增加 len<96 越界防护（防服务崩溃）；单实例互斥句柄改 Box+forget 显式持有；LAST_CPU 的 Mutex 改双 AtomicU64 + swap（单线程采样无锁）；log/log_cont 合并为 write_log 统一出口，文件级 use io::Write；clear_caches 三处裸 NtSetSystemInformation 封装为 nt_set 统一辅助函数（set_memory_command 复用），日志统一为 `NtSetSystemInformation(信息类)` 格式；枚举进程循环改 loop+break 消除 clippy while_immutable_condition；clippy 0 警告
- 2026-08-28：项目更名 Hydride → Scandium（目录已改）——包名/二进制 scandium_svc64.exe、单实例互斥 Global\Scandium_WRCS_SingleInstance、服务名/TOML/.osiml 同步、安装包 scandium-svc-win-x64-setup、仓库 URL 与徽章 NXRKYMANE/Scandium、镜像同步参数（sync-releases.ps1 缓存目录 scandium-sync-cache + release-sync.yml 仓库名）、默认安装目录 {autopf}\Scandium；保留 AppId GUID、NSIS 旧卸载键与旧目录清理判断中的 Hydride 历史引用（升级兼容）；README 双语 + CONTRIBUTING + SECURITY + bug_report + BUILD.ps1 + build.rs 同步；git 双远端 github（GitHub）+ origin（私有 Gitea git.nxrkymane.monster）
- 2026-08-21：清理引擎升级为「排名前二工具」交叉逻辑（源自 Mem Reduct + WinMemoryCleaner）——工作集引擎改内核级 MemoryEmptyWorkingSets(2)（失败回退逐进程修剪）；缓存引擎从单 Standby purge 升级为清理链：PurgeStandby(4)+LowPriorityStandby(5)+CombinePhysicalMemory(130,16B)+SystemFileCache(21,64B)+RegistryReconciliation(155)；实测单次缓存释放 743MB（原 631MB）、工作集 261MB；CPU≥85% 时两引擎均整周期暂停；README 双语同步；SystemFileCache 需服务 LocalSystem 账户
- 2026-08-21：效率模式（EcoQoS，源自 Osmium）——installer.iss 生成的 TOML 新增 eco_qos/host_eco_qos（均 auto）及 4 个 CPU 阈值字段，服务与宿主按负载自动进出任务管理器效率模式；README 双语新增「效率模式」章节，调整阈值需改 .osiml 后 os.exe --refresh
- 2026-08-21：服务显示名改为 Windows RAM Clean Service（installer.iss VersionInfoProductName + TOML service_display_name、service_core.rs 启动日志）；README 徽章区与 Osmium 项目统一（白色系 Rust/Gitee/AtomGit/Douyin/Views 徽章，移除 QQ 徽章与 komarev）
- 2026-08-21：Release 资产镜像同步（源自 Osmium 项目）——复制 .github/scripts/sync-releases.ps1（缓存目录改 hydride-sync-cache）与 .github/workflows/release-sync.yml（仓库参数 NXRKYMANE/Hydride），Gitee + AtomGit 双镜像，触发：release 事件 + 手动 + 每日 4 点兜底；需仓库配置 GITEE_TOKEN / ATOMGIT_TOKEN secrets
- 2026-08-21：调度加入 CPU 感知门控——每周期用 GetSystemTimes 采样系统 CPU，≥30% 降 1 档、≥60% 降 2 档、≥85% 整周期暂停（内存档 1~5 次/分不变），日志新增 CPU 列（`Mem 41.6% | CPU 51% → WorkingSet 1 run(s)/min`）；同时修复 get_standby_mb 首个 0 长度查询误判失败返回 0 的 bug（NtQuerySystemInformation 长度不足必然返回非 0，只取 len）；README 双语同步
- 2026-08-20：脱离外部引擎——删除 Libs/LIBPCL2.dll 与 wrcs.exe 全部解码/还原/进程管理代码（decode_wrcs/ensure_wrcs/run_wrcs/kill_tree/CHILD_PIDS），改为原生工作集修剪引擎：Toolhelp32 枚举进程 + SetProcessWorkingSetSize(-1,-1)（EmptyWorkingSet）+ 保留 NtSetSystemInformation 清 Standby；Cargo.toml 移除 base64 依赖；installer.iss 移除 Libs 复制、TOML 配置改写到 {app} 根目录；日志格式不变（Used/Standby）；实测释放 219MB（14923→14704MB）；README 双语 + SECURITY.md + CLAUDE.md 同步；版本升至 2.6.0
- 2026-08-18：清理引擎更名——临时工作目录 HSMM → WRCS（`%WINDIR%\Temp\WRCS`）、hsmmts.exe → wrcs.exe（函数 decode_hsmmts/ensure_hsmmts/run_hsmmts → decode_wrcs/ensure_wrcs/run_wrcs、单实例互斥 Global\Hydride_WRCS_SingleInstance）；README 双语 + SECURITY.md + CLAUDE.md 同步
- 2026-08-18：体积极限优化——Cargo.toml opt-level 改 "z"（体积优先），BUILD.ps1 新增 UPX `--ultra-brute --lzma` 极限压缩步骤（Publish\hydride_svc64.exe 就地压缩后进安装包，UPX 路径 F:\DevTools\UPX\upx.exe，缺失即报错）；实测 295,936 → 152,064 B（51.4%）；README 双语标题去 🧠 emoji、体积描述同步 ~150 KB；SECURITY.md 同步
- 2026-08-18：版本升至 2.4.0；安装流程去 --delete——PrepareToInstall 停止旧服务改 `--stop hydride_svc64`（--delete 会连同 svcs 日志删除；配置更新由 --install 完成），卸载流程保留 --delete；ISCC 编译通过
- 2026-08-17：前置框架迁移适配 Silanes → Osmium（v26.7.0 品牌重命名）——installer.iss 注册表键 `App Paths\os.exe`、函数 SilanesExec/RunSilanesCommand → OsmiumExec/RunOsmiumCommand、消息 SilanesNotFound → OsmiumNotFound（URL 指 NXRKYMANE/Osmium）；README 双语 + SECURITY.md + bug_report.md 日志路径同步（`ProgramData\Osmium\svcs`）；服务注册命令不变（--install/--start/--delete）；ISCC 编译验证通过
- 2026-08-10：目录重构 rust→Project、libs→Libs、misc→Misc、publish→根目录 Publish；Docs 文件夹已删除
- 2026-08-10：服务配置由 YAML 迁移为 TOML（installer.iss 生成 hydride_svc64.toml，路径用单引号字面字符串）
- 2026-08-10：BUILD.ps1 固化工具链（无 VS 时自动配置 F:\DevTools\MSVC + Windows11 SDK，并自动探测 Inno Setup 6/7）
- 2026-08-10：版本升至 2.2.0；git 提交历史、GitHub Release 页面、GitHub 平台文件（CONTRIBUTING/SECURITY/.github 等）全部英文化；SECURITY.md 去除版本号
- 2026-08-10：发布 v2.2.0（提交 2035d33 + tag + Release，资产为安装包，说明英文，格式与往期一致）
- 2026-08-10：重写历史提交消息去除全部版本号（如 v3.6.2、.NET 10、v2.2.0）并强推；提交消息一律英文且不含版本号
- 2026-08-11：修复调度塌缩 bug——低内存档（PCL2 1 次/分）时周期循环任务瞬间完成即退出外层循环，导致每秒清理；现内层循环跑满 60s 周期，已完成引擎等待到周期结束
- 2026-08-11：清理频率调整——PCL2 引擎上限升至 5 次/分（每 25% 一档），Standby 固定 1 次/分；版本升至 2.3.0
- 2026-08-11：发布 v2.3.0（提交 dfe7f99 + tag + Release，资产为安装包，说明英文，格式与往期一致）
