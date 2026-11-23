//! Resource meter on the top bar: how much CPU and memory the whole app uses, counting the
//! app's own process and every WebView2 process it started (games, streams, the app's pages),
//! the way Task Manager adds them up under the app.
//!
//! - CPU: the processes' busy time over the last interval, as a share of the whole machine
//!   (all cores = 100 %), like Task Manager's CPU column.
//! - Memory: their private working sets ("Memory" in Task Manager), so pages shared between
//!   processes aren't counted twice. Windows versions without that number use private bytes.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// How often the meter is updated.
const EVERY: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct Usage {
    /// Share of the whole machine, 0–100.
    pub cpu: f64,
    /// Bytes.
    pub memory: u64,
    /// How many processes were counted.
    pub processes: usize,
}

#[cfg(windows)]
#[repr(C)]
struct ProcessEntry32W {
    size: u32,
    usage: u32,
    pid: u32,
    default_heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    priority: i32,
    flags: u32,
    exe: [u16; 260],
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct MemoryCounters {
    cb: u32,
    page_faults: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged: usize,
    quota_paged: usize,
    quota_peak_non_paged: usize,
    quota_non_paged: usize,
    pagefile: usize,
    peak_pagefile: usize,
    private_usage: usize,
    private_working_set: usize,
    shared_commit: u64,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
impl FileTime {
    fn ticks(self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}

#[cfg(windows)]
const TH32CS_SNAPPROCESS: u32 = 0x2;
#[cfg(windows)]
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
#[cfg(windows)]
const INVALID_HANDLE: isize = -1;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(handle: isize) -> i32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn GetProcessTimes(process: isize, creation: *mut FileTime, exit: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    fn K32GetProcessMemoryInfo(process: isize, counters: *mut MemoryCounters, cb: u32) -> i32;
}

#[cfg(windows)]
/// Every running process as (pid, parent pid).
fn process_parents() -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE || snapshot == 0 {
            return out;
        }
        let mut entry: ProcessEntry32W = std::mem::zeroed();
        entry.size = std::mem::size_of::<ProcessEntry32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            out.push((entry.pid, entry.parent));
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    out
}

/// `root` and everything it started, directly or not.
pub fn tree(root: u32, parents: &[(u32, u32)]) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(pid, parent) in parents {
        if pid != parent {
            children.entry(parent).or_default().push(pid);
        }
    }
    let mut seen = HashSet::from([root]);
    let mut todo = vec![root];
    while let Some(pid) = todo.pop() {
        for &child in children.get(&pid).map(Vec::as_slice).unwrap_or(&[]) {
            if seen.insert(child) {
                todo.push(child);
            }
        }
    }
    let mut out: Vec<u32> = seen.into_iter().collect();
    out.sort_unstable();
    out
}

#[cfg(windows)]
/// One process's CPU time so far (100 ns ticks) and memory (bytes), if it can be read.
fn read_process(pid: u32) -> Option<(u64, u64)> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return None;
        }
        let (mut created, mut exited, mut kernel, mut user) =
            (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
        let times = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) != 0;
        let mut counters = MemoryCounters { cb: std::mem::size_of::<MemoryCounters>() as u32, ..Default::default() };
        let mut memory = 0u64;
        if K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) != 0 {
            memory = counters.private_working_set as u64;
        } else {
            // Older Windows: the shorter struct (PROCESS_MEMORY_COUNTERS_EX, without the last two
            // fields), private bytes instead.
            let cb = (std::mem::size_of::<MemoryCounters>() - std::mem::size_of::<usize>() - 8) as u32;
            counters = MemoryCounters { cb, ..Default::default() };
            if K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) != 0 {
                memory = counters.private_usage as u64;
            }
        }
        CloseHandle(handle);
        times.then(|| (kernel.ticks() + user.ticks(), memory))
    }
}

// Linux: the same from /proc. CPU times come in clock ticks (USER_HZ, 100 a second on every
// common kernel) and are turned into 100 ns ticks like Windows'; memory is the private part of
// each process (Private_Clean + Private_Dirty of smaps_rollup), or its RSS when that's missing.
#[cfg(target_os = "linux")]
const TICKS_PER_JIFFY: u64 = 100_000;

/// The fields after the process name in /proc/<pid>/stat (the name may hold spaces).
#[cfg(target_os = "linux")]
fn stat_fields(pid: u32) -> Option<Vec<String>> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &stat[stat.rfind(')')? + 2..];
    Some(rest.split_whitespace().map(str::to_string).collect())
}

#[cfg(target_os = "linux")]
fn process_parents() -> Vec<(u32, u32)> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
    dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(|pid| {
            // Field 4 of stat (index 1 after the name) is the parent.
            let parent = stat_fields(pid)?.get(1)?.parse().ok()?;
            Some((pid, parent))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn read_process(pid: u32) -> Option<(u64, u64)> {
    let fields = stat_fields(pid)?;
    // utime and stime: fields 14 and 15 (indexes 11 and 12 after the name).
    let jiffies: u64 = fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
    let kb = |text: &str, keys: &[&str]| -> u64 {
        text.lines()
            .filter(|l| keys.iter().any(|k| l.starts_with(k)))
            .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
            .sum()
    };
    let memory_kb = match std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")) {
        Ok(text) => kb(&text, &["Private_Clean:", "Private_Dirty:"]),
        Err(_) => kb(&std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?, &["VmRSS:"]),
    };
    Some((jiffies * TICKS_PER_JIFFY, memory_kb * 1024))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_parents() -> Vec<(u32, u32)> {
    Vec::new()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn read_process(_pid: u32) -> Option<(u64, u64)> {
    None
}

/// Measures the app every couple of seconds and sends it to the top bar ("resources").
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let me = std::process::id();
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
        let mut last: HashMap<u32, u64> = HashMap::new();
        let mut last_at = Instant::now();
        loop {
            std::thread::sleep(EVERY);
            let pids = tree(me, &process_parents());
            let now = Instant::now();
            let elapsed = now.duration_since(last_at).as_secs_f64();
            last_at = now;
            let mut busy = 0u64;
            let mut usage = Usage::default();
            let mut seen: HashMap<u32, u64> = HashMap::new();
            for pid in pids {
                let Some((cpu, memory)) = read_process(pid) else { continue };
                // A process new since the last sample only counts from now on.
                if let Some(before) = last.get(&pid) {
                    busy += cpu.saturating_sub(*before);
                }
                seen.insert(pid, cpu);
                usage.memory += memory;
                usage.processes += 1;
            }
            let first = last.is_empty();
            last = seen;
            if first || elapsed <= 0.0 {
                continue;
            }
            // Ticks are 100 ns.
            usage.cpu = ((busy as f64 / 1e7) / elapsed / cores * 100.0).clamp(0.0, 100.0);
            let _ = app.emit_to("toolbar", "resources", usage);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_follows_children_and_grandchildren_only() {
        // 1 → 2 → 3, 1 → 4; 5 is someone else's; 6 claims to be its own parent.
        let parents = [(1, 0), (2, 1), (3, 2), (4, 1), (5, 9), (6, 6)];
        assert_eq!(tree(1, &parents), vec![1, 2, 3, 4]);
        assert_eq!(tree(5, &parents), vec![5]);
        assert_eq!(tree(6, &parents), vec![6]);
    }

    #[test]
    fn reads_this_process() {
        let (cpu, memory) = read_process(std::process::id()).expect("own process");
        let _ = cpu;
        assert!(memory > 0);
        assert!(tree(std::process::id(), &process_parents()).contains(&std::process::id()));
    }
}
