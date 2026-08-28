// Windows 原生结构体沿用其官方命名（MEMORYSTATUSEX 等全大写缩写）
#![allow(clippy::upper_case_acronyms)]

use std::ffi::c_void;
use std::io::Write;
use std::mem::forget;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// ==================== 常量与全局状态 ====================

const CYCLE_MS: u64 = 60_000;
const SE_PROFILE_SINGLE_PROCESS: u32 = 13; // 内存列表命令（清 Standby 等）所需特权
const SE_INCREASE_QUOTA: u32 = 5; // 清系统文件缓存（SystemFileCacheInformation）所需特权
const SYSTEM_MEMORY_LIST_INFO: i32 = 80; // NtSetSystemInformation 信息类
const SYSTEM_FILE_CACHE_INFO: i32 = 21; // 清系统文件缓存工作集
const SYSTEM_REGISTRY_RECON_INFO: i32 = 155; // 清注册表缓存（win8.1+）
const SYSTEM_COMBINE_PHYS_MEM_INFO: i32 = 130; // 合并物理内存列表（win10+）
const MEMORY_EMPTY_WORKING_SETS: i32 = 2; // 内核级清空全部进程工作集
const MEMORY_FLUSH_MODIFIED_LIST: i32 = 3; // 脏页写回磁盘（页文件），落入 Standby 后才能被清除
const MEMORY_PURGE_STANDBY_LIST: i32 = 4; // 清空 Standby 列表命令
const MEMORY_PURGE_LOW_PRIORITY_STANDBY: i32 = 5; // 清空低优先级 Standby 列表
const TH32CS_SNAPPROCESS: u32 = 0x0000_0002; // 进程快照
const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const PROCESS_SET_QUOTA: u32 = 0x0100; // 修剪工作集所需权限
const MAX_PATH: usize = 260;
const CPU_REDUCE_HIGH: f64 = 30.0; // CPU ≥30% 降 1 档
const CPU_REDUCE_HEAVY: f64 = 60.0; // CPU ≥60% 降 2 档
const CPU_PAUSE: f64 = 85.0; // CPU ≥85% 本周期暂停
const DISK_BUSY_SKIP: f64 = 60.0; // 磁盘 ≥60% 忙时跳过脏页写回（保护机械盘）
const FILE_CACHE_MIN_MEM: f64 = 50.0; // 内存 <50% 时不清文件缓存（收益低于性能损失）
const STANDBY_HIGH_MEM: f64 = 80.0; // 内存 ≥80% 时 Standby 链提速到 2 次/分

static STOP: AtomicBool = AtomicBool::new(false);
// 上次 (idle, total) CPU 采样：主循环单线程读写，原子仅作跨调用状态保存
static LAST_CPU_IDLE: AtomicU64 = AtomicU64::new(0);
static LAST_CPU_TOTAL: AtomicU64 = AtomicU64::new(0);
// PDH 磁盘活动计数器句柄（进程生命周期持有，0 表示初始化失败已退化）
static DISK_QUERY: AtomicUsize = AtomicUsize::new(0);
static DISK_COUNTER: AtomicUsize = AtomicUsize::new(0);

// ==================== Windows FFI ====================

#[repr(C)]
struct MEMORYSTATUSEX {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
}

#[repr(C)]
struct PROCESSENTRY32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; MAX_PATH],
}

#[repr(C)]
struct FILETIME {
    dw_low_date_time: u32,
    dw_high_date_time: u32,
}

// NtSetSystemInformation(SystemFileCacheInformation) 参数：64 字节结构，前 16 字节工作集上下限置 -1 清空文件缓存
#[repr(C)]
struct SystemFilecacheInformation {
    ul_minimum_working_set: usize,
    ul_maximum_working_set: usize,
    _reserved: [u8; 48],
}

// NtSetSystemInformation(SystemCombinePhysicalMemoryInformation) 参数：Handle=0 + 空页数组触发合并（16 字节）
#[repr(C)]
struct SystemMemoryCombineInformationEx {
    handle: *mut c_void,
    pages: [usize; 1],
}

// PdhGetFormattedCounterValue(PDH_FMT_DOUBLE) 返回值：CStatus + double 占用率
#[repr(C)]
struct PdhFmtCounterValue {
    _c_status: u32,
    double_value: f64,
}

const PDH_FMT_DOUBLE: u32 = 0x0000_0200; // PDH 取值格式：double
const ERROR_SUCCESS: i32 = 0;

unsafe extern "system" {
    fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> i32;
    fn CreateMutexW(attributes: *const c_void, initial_owner: i32, name: *const u16)
    -> *mut c_void;
    fn GetLastError() -> u32;
    fn NtSetSystemInformation(info_class: i32, info: *const c_void, len: u32) -> i32;
    fn NtQuerySystemInformation(
        info_class: i32,
        info: *mut c_void,
        len: u32,
        ret_len: *mut u32,
    ) -> i32;
    fn RtlAdjustPrivilege(
        privilege: u32,
        enable: u8,
        current_thread: u8,
        old_value: *mut u8,
    ) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snapshot: isize, entry: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(snapshot: isize, entry: *mut PROCESSENTRY32W) -> i32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn SetProcessWorkingSetSize(handle: *mut c_void, min: usize, max: usize) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn GetSystemTimes(idle: *mut FILETIME, kernel: *mut FILETIME, user: *mut FILETIME) -> i32;
}

// PDH：磁盘活动采样（% Disk Time），需单独链接 pdh.lib
#[link(name = "pdh")]
unsafe extern "system" {
    fn PdhOpenQueryW(data_source: *const c_void, user_data: usize, query: *mut *mut c_void) -> i32;
    fn PdhAddEnglishCounterW(
        query: *mut c_void,
        path: *const u16,
        user_data: usize,
        counter: *mut *mut c_void,
    ) -> i32;
    fn PdhCollectQueryData(query: *mut c_void) -> i32;
    fn PdhGetFormattedCounterValue(
        counter: *mut c_void,
        format: u32,
        item_type: *mut u32,
        value: *mut PdhFmtCounterValue,
    ) -> i32;
    fn PdhCloseQuery(query: *mut c_void) -> i32;
}

// ==================== 调度引擎 ====================

// 轻量引擎：周期内按各自预算（runs/分）均匀触发，互不阻塞
struct Engine {
    runs: u32,
    interval: u64,
    next: u64,
    done: u32,
    task: fn(),
}

impl Engine {
    fn new(runs: u32, offset_ms: u64, task: fn()) -> Self {
        Engine {
            runs,
            interval: CYCLE_MS / u64::from(runs.max(1)),
            next: offset_ms,
            done: 0,
            task,
        }
    }
}

// ==================== 启动 ====================

pub fn main_entry() {
    if !acquire_single_instance() {
        log("ERROR: Another Scandium instance is already running. Exiting.");
        return;
    }

    enable_privileges();
    init_disk_counter();

    log("Windows RAM Clean Service started (Press Ctrl+C to exit)");

    // Ctrl+C 置位停止标志，主循环据此退出
    ctrlc::set_handler(|| STOP.store(true, Ordering::SeqCst)).ok();

    // 主服务循环：四引擎按 CPU/内存/磁盘采样独立定档，同周期内交错执行
    while !STOP.load(Ordering::SeqCst) {
        let mem_pct = get_memory_percent();
        let cpu_pct = get_cpu_percent();
        let disk_pct = get_disk_busy_percent();
        let paused = cpu_pct >= CPU_PAUSE;

        // 工作集引擎：内存分档提速（<50% 仅保底 1 次/分：低占用下换出页会立即回读，无净收益）
        let mem_runs: i32 = if mem_pct < 50.0 {
            1
        } else if mem_pct < 70.0 {
            2
        } else if mem_pct < 85.0 {
            3
        } else if mem_pct < 95.0 {
            4
        } else {
            5
        };
        let ws_runs = if paused {
            0
        } else if cpu_pct >= CPU_REDUCE_HEAVY {
            (mem_runs - 2).max(1)
        } else if cpu_pct >= CPU_REDUCE_HIGH {
            (mem_runs - 1).max(1)
        } else {
            mem_runs
        };
        // Standby 链：内存 ≥80% 提速到 2 次/分，CPU 极高暂停
        let standby_runs = if paused {
            0
        } else if mem_pct >= STANDBY_HIGH_MEM {
            2
        } else {
            1
        };
        // 文件缓存引擎：仅内存充足时清理（<50% 时清理收益低于文件性能损失）
        let file_cache_runs = if paused || mem_pct < FILE_CACHE_MIN_MEM {
            0
        } else {
            1
        };
        // 维护引擎：固定 1 次/分，仅受 CPU 极高暂停
        let maint_runs = u32::from(!paused);

        let fmt_runs = |runs: u32| {
            if runs == 0 {
                "paused".to_string()
            } else {
                runs.to_string()
            }
        };
        log(&format!(
            "Mem {mem_pct:.1}% | CPU {cpu_pct:.0}% | Disk {disk_pct:.0}% → WS {}, Standby {}, FileCache {}, Maint {} run(s)/min",
            fmt_runs(ws_runs as u32),
            fmt_runs(standby_runs),
            fmt_runs(file_cache_runs),
            fmt_runs(maint_runs),
        ));

        // 周期内循环：四引擎按各自节奏交错执行，跑满 60s 后重新定档
        let start = Instant::now();
        let start_mb = get_used_memory_mb();
        let mut engines = [
            Engine::new(ws_runs as u32, 0, run_cleanup_once),
            Engine::new(standby_runs, 10_000, purge_standby_chain),
            Engine::new(file_cache_runs, 20_000, clear_file_cache),
            Engine::new(maint_runs, 30_000, maintain_lists),
        ];
        while !STOP.load(Ordering::SeqCst) {
            let elapsed = start.elapsed().as_millis() as u64;

            for engine in &mut engines {
                if engine.done < engine.runs && elapsed >= engine.next {
                    (engine.task)();
                    engine.done += 1;
                    engine.next += engine.interval;
                }
            }

            // 已到周期末尾：进入下一周期
            if elapsed >= CYCLE_MS {
                break;
            }

            // 等待最早的下一个触发点（已完成的引擎等到周期结束；每次最多 1 秒）
            let wait_until = engines
                .iter()
                .filter(|e| e.done < e.runs)
                .map(|e| e.next)
                .min()
                .unwrap_or(CYCLE_MS);
            let wait_ms = wait_until.saturating_sub(elapsed).min(1000);
            std::thread::sleep(Duration::from_millis(wait_ms));
        }

        // 周期汇总：对比整周期前后内存占用，作为各引擎分项日志之外的总体参照
        let end_mb = get_used_memory_mb();
        log(&format!(
            "Cycle: Mem {mem_pct:.1}% → {:.1}% ({})",
            get_memory_percent(),
            mem_delta_str(start_mb, end_mb, true)
        ));

        // 周期之间输出空行分隔（复用 log 的实时 flush）
        log("");
    }

    log("Service stopped.");
}

// ==================== 基础工具 ====================

/// 日志输出（立即 flush，保证服务日志实时落盘；时间戳由服务宿主统一添加）
fn log(msg: &str) {
    write_log(msg, false);
}

/// 缩进续行日志（清理结果行）
fn log_cont(msg: &str) {
    write_log(msg, true);
}

/// 日志统一出口：可选缩进，立即 flush
fn write_log(msg: &str, indent: bool) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{}{msg}", if indent { "   " } else { "" });
    let _ = lock.flush();
}

/// 单实例互斥：作为服务应全局唯一，避免多个实例同时清理
fn acquire_single_instance() -> bool {
    let name: Vec<u16> = "Global\\Scandium_WRCS_SingleInstance"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() || unsafe { GetLastError() } == 183 {
        return false; // ERROR_ALREADY_EXISTS
    }
    // Box 持有句柄并 forget：句柄存活到进程退出（系统自动回收），锁保持有效
    forget(Box::new(handle));
    true
}

/// 启用清理所需特权（管理员令牌中默认禁用），失败仅记日志：
/// SeProfileSingleProcess 供内存列表命令，SeIncreaseQuota 供清系统文件缓存
fn enable_privileges() {
    for id in [SE_PROFILE_SINGLE_PROCESS, SE_INCREASE_QUOTA] {
        let mut old = 0u8;
        let status = unsafe { RtlAdjustPrivilege(id, 1, 0, &mut old) };
        if status != 0 {
            log(&format!(
                "WARN: RtlAdjustPrivilege({id}) failed: NTSTATUS 0x{status:08X} (not running as administrator?)"
            ));
        }
    }
}

// ==================== 内存状态 ====================

fn get_memory_status() -> MEMORYSTATUSEX {
    let mut mem = MEMORYSTATUSEX {
        dw_length: size_of::<MEMORYSTATUSEX>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
        ull_total_page_file: 0,
        ull_avail_page_file: 0,
        ull_total_virtual: 0,
        ull_avail_virtual: 0,
        ull_avail_extended_virtual: 0,
    };
    unsafe { GlobalMemoryStatusEx(&mut mem) };
    mem
}

/// 当前物理内存使用率（0-100）
fn get_memory_percent() -> f64 {
    let mem = get_memory_status();
    (mem.ull_total_phys - mem.ull_avail_phys) as f64 / mem.ull_total_phys as f64 * 100.0
}

/// 当前已使用物理内存（MB）
fn get_used_memory_mb() -> u64 {
    let mem = get_memory_status();
    (mem.ull_total_phys - mem.ull_avail_phys) / 1024 / 1024
}

/// 生成内存对比片段："A → B (±NMB)"；with_pct 为 true 时附带变化百分比与方向箭头
fn mem_delta_str(before: u64, after: u64, with_pct: bool) -> String {
    let delta = before as i64 - after as i64;
    let arrow = if delta >= 0 { "↓" } else { "↑" };
    let pct = if with_pct {
        let p = if before > 0 {
            delta.unsigned_abs() as f64 / before as f64 * 100.0
        } else {
            0.0
        };
        format!(", {p:.1}%{arrow}")
    } else {
        String::new()
    };
    format!(
        "{before}MB → {after}MB ({}{}MB{pct})",
        if delta >= 0 { "+" } else { "-" },
        delta.unsigned_abs()
    )
}

/// 系统 CPU 使用率（0-100），基于两次采样差值；首次采样返回 0
fn get_cpu_percent() -> f64 {
    let mut idle = FILETIME {
        dw_low_date_time: 0,
        dw_high_date_time: 0,
    };
    let mut kernel = FILETIME {
        dw_low_date_time: 0,
        dw_high_date_time: 0,
    };
    let mut user = FILETIME {
        dw_low_date_time: 0,
        dw_high_date_time: 0,
    };
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return 0.0;
    }

    let to_u64 = |ft: FILETIME| (ft.dw_high_date_time as u64) << 32 | ft.dw_low_date_time as u64;
    let now_idle = to_u64(idle);
    // kernel 时间已包含 idle，总时长 = kernel + user，勿把 idle 重复计入
    let now_total = to_u64(kernel) + to_u64(user);

    // 交换旧值并写入新值：首次采样或时间倒退时无法计算
    let last_idle = LAST_CPU_IDLE.swap(now_idle, Ordering::Relaxed);
    let last_total = LAST_CPU_TOTAL.swap(now_total, Ordering::Relaxed);
    if last_total == 0 || now_total <= last_total {
        return 0.0;
    }
    let idle_delta = now_idle - last_idle;
    let total_delta = now_total - last_total;
    if total_delta == 0 {
        return 0.0;
    }
    (1.0 - idle_delta as f64 / total_delta as f64) * 100.0
}

/// 初始化磁盘活动计数器（PhysicalDisk(_Total)\% Disk Time），失败记 WARN 并退化为不拦截
/// （新版 Windows 11 上 % Active Time 计数器已不存在，故采用 % Disk Time）
fn init_disk_counter() {
    let path: Vec<u16> = "\\PhysicalDisk(_Total)\\% Disk Time"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut query = std::ptr::null_mut();
    let mut counter = std::ptr::null_mut();
    unsafe {
        if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != ERROR_SUCCESS {
            log("WARN: Disk counter init failed at PdhOpenQueryW (gating disabled)");
            return;
        }
        if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != ERROR_SUCCESS {
            PdhCloseQuery(query);
            log("WARN: Disk counter init failed at PdhAddEnglishCounterW (gating disabled)");
            return;
        }
        // 首次采集作为基线，之后每次采集与上次的差值即区间占用率
        PdhCollectQueryData(query);
    }
    DISK_QUERY.store(query as usize, Ordering::Relaxed);
    DISK_COUNTER.store(counter as usize, Ordering::Relaxed);
}

/// 磁盘活动占用率（0-100），基于 PDH 差值采样；未初始化或失败返回 0（不拦截写回）
fn get_disk_busy_percent() -> f64 {
    let query = DISK_QUERY.load(Ordering::Relaxed);
    let counter = DISK_COUNTER.load(Ordering::Relaxed);
    if query == 0 || counter == 0 {
        return 0.0;
    }
    let mut value = PdhFmtCounterValue {
        _c_status: 0,
        double_value: 0.0,
    };
    unsafe {
        if PdhCollectQueryData(query as *mut c_void) != ERROR_SUCCESS {
            return 0.0;
        }
        if PdhGetFormattedCounterValue(
            counter as *mut c_void,
            PDH_FMT_DOUBLE,
            std::ptr::null_mut(),
            &mut value,
        ) != ERROR_SUCCESS
        {
            return 0.0;
        }
    }
    value.double_value.clamp(0.0, 100.0)
}

/// 当前 Standby 缓存大小（MB），失败返回 0
fn get_standby_mb() -> u64 {
    // 先以 0 长度查询所需缓冲大小（必然返回长度不足，仅取 len），再按字段偏移读取
    let mut len = 0u32;
    let _ = unsafe {
        NtQuerySystemInformation(SYSTEM_MEMORY_LIST_INFO, std::ptr::null_mut(), 0, &mut len)
    };
    // 缓冲须覆盖 6 个缓存字段（StandbyPageCount 起 48 字节）才可安全解析
    if len < 96 {
        return 0;
    }

    let mut buf = vec![0u8; len as usize];
    let status = unsafe {
        NtQuerySystemInformation(
            SYSTEM_MEMORY_LIST_INFO,
            buf.as_mut_ptr() as *mut c_void,
            len,
            &mut len,
        )
    };
    if status != 0 {
        return 0;
    }

    // StandbyPageCount 为第 7 个 ULONG_PTR（偏移 48），随后 5 类缓存细分；每页 4KB
    let read_u64 = |off: usize| u64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
    let pages =
        read_u64(48) + read_u64(56) + read_u64(64) + read_u64(72) + read_u64(80) + read_u64(88);
    pages * 4 / 1024
}

// ==================== 工作集清理引擎 ====================

/// 内核级清空全部进程工作集（一条调用），失败（无特权）则回退到逐进程修剪
fn trim_working_sets() {
    if !set_memory_command(MEMORY_EMPTY_WORKING_SETS) {
        trim_working_sets_by_enumeration();
    }
}

/// 内核级内存列表命令（NtSetSystemInformation(SystemMemoryListInformation)），成功返回 true
fn set_memory_command(command: i32) -> bool {
    nt_set(
        SYSTEM_MEMORY_LIST_INFO,
        &command as *const i32 as *const c_void,
        size_of::<i32>() as u32,
    )
}

/// 统一调用 NtSetSystemInformation：失败记日志并返回 false
fn nt_set(info_class: i32, info: *const c_void, len: u32) -> bool {
    let status = unsafe { NtSetSystemInformation(info_class, info, len) };
    if status != 0 {
        log(&format!(
            "NtSetSystemInformation({info_class}) failed: NTSTATUS 0x{status:08X}"
        ));
        return false;
    }
    true
}

/// 遍历系统进程，对每个进程修剪工作集（EmptyWorkingSet），失败静默跳过
fn trim_working_sets_by_enumeration() {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == -1 {
        log("Failed to enumerate processes (CreateToolhelp32Snapshot)");
        return;
    }

    let mut entry = PROCESSENTRY32W {
        dw_size: size_of::<PROCESSENTRY32W>() as u32,
        cnt_usage: 0,
        th32_process_id: 0,
        th32_default_heap_id: 0,
        th32_module_id: 0,
        cnt_threads: 0,
        th32_parent_process_id: 0,
        pc_pri_class_base: 0,
        dw_flags: 0,
        sz_exe_file: [0; MAX_PATH],
    };

    if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
        loop {
            if entry.th32_process_id != 0 {
                trim_working_set(entry.th32_process_id);
            }
            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    let _ = unsafe { CloseHandle(snapshot as *mut c_void) };
}

/// 对单个进程调用 SetProcessWorkingSetSize(-1, -1)（EmptyWorkingSet），权限不足或系统进程自动跳过
fn trim_working_set(pid: u32) {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, 0, pid) };
    if handle.is_null() {
        return;
    }
    let _ = unsafe { SetProcessWorkingSetSize(handle, usize::MAX, usize::MAX) };
    let _ = unsafe { CloseHandle(handle) };
}

/// 执行一次工作集清理（内核级 + 遍历兜底），按 Used 格式输出日志
fn run_cleanup_once() {
    let before = get_used_memory_mb();

    trim_working_sets();

    let after = get_used_memory_mb();
    log_cont(&format!("Used: {}", mem_delta_str(before, after, false)));
}

// ==================== 缓存清理引擎（Standby / 文件缓存 / 维护） ====================

/// Standby 链：脏页写回磁盘（受磁盘门控）→ 清 Standby → 清低优先级 Standby，按 Standby 格式输出日志
fn purge_standby_chain() {
    let before = get_standby_mb();

    // 磁盘高负载时跳过脏页写回（纯磁盘 I/O，保护机械盘），仅清理已干净的 Standby 页
    let note = if get_disk_busy_percent() >= DISK_BUSY_SKIP {
        ", dirty-flush skipped (disk busy)"
    } else {
        set_memory_command(MEMORY_FLUSH_MODIFIED_LIST);
        ""
    };
    set_memory_command(MEMORY_PURGE_STANDBY_LIST);
    set_memory_command(MEMORY_PURGE_LOW_PRIORITY_STANDBY);

    let after = get_standby_mb();
    let freed = before.saturating_sub(after);
    log_cont(&format!(
        "Standby: {before}MB → {after}MB (freed {freed}MB{note})"
    ));
}

/// 文件缓存引擎：工作集上下限置 -1 触发清空（64 字节结构，需 SeIncreaseQuota 特权，启动时已启用）
fn clear_file_cache() {
    let cache_info = SystemFilecacheInformation {
        ul_minimum_working_set: usize::MAX,
        ul_maximum_working_set: usize::MAX,
        _reserved: [0; 48],
    };
    if nt_set(
        SYSTEM_FILE_CACHE_INFO,
        &cache_info as *const SystemFilecacheInformation as *const c_void,
        size_of::<SystemFilecacheInformation>() as u32,
    ) {
        log_cont("FileCache: cleared");
    }
}

/// 维护引擎：注册表缓存整理（win8.1+）+ 合并物理内存列表（win10+），失败仅记日志
fn maintain_lists() {
    let reg_ok = nt_set(SYSTEM_REGISTRY_RECON_INFO, std::ptr::null(), 0);
    let combine = SystemMemoryCombineInformationEx {
        handle: std::ptr::null_mut(),
        pages: [0],
    };
    let combine_ok = nt_set(
        SYSTEM_COMBINE_PHYS_MEM_INFO,
        &combine as *const SystemMemoryCombineInformationEx as *const c_void,
        size_of::<SystemMemoryCombineInformationEx>() as u32,
    );
    if reg_ok && combine_ok {
        log_cont("Maint: registry reconciliation + memory combine ok");
    }
}
