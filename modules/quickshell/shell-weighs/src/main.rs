//! What the shell weighs, part by part: `shell-weighs` prints one line for
//! the journal, read from outside the shell so the shell's own thread pays
//! nothing for it. A timer runs it every ten minutes (home.nix), so the
//! journal says how the shell's memory grows between switches and what a cut
//! saved.
//!
//! It reads only files: the shell's service's cgroup for its processes and
//! its whole charge, each process's /proc status, the shell's smaps for its
//! parts, and the fdinfo of its render node and dma-bufs for what the GPU
//! holds for it. Nothing is attached to, so it's safe on a shell in use.
//!
//! One number never said it: on 2026-11-11 the service was charged 986 MB
//! while the shell itself held 651, the rest being the GPU's 253 MB in
//! system memory and an 87 MB helper, and of the shell's own the JavaScript
//! heap swung between 200 and 307 MB within minutes.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The shell's resident memory by part, in kB, from its smaps.
#[derive(Debug, Default, PartialEq, Eq)]
struct Parts {
    /// Qt's JavaScript heap, the memfd it names `JSGCHeap`.
    heap: u64,
    /// The code Qt's JIT made, the memfd it names `JITCode`.
    jit: u64,
    /// Every other anonymous page: malloc's, Qt's items, decoded images.
    anonymous: u64,
    /// Pages of files mapped in: libraries and fonts, which the kernel may
    /// drop and read again.
    files: u64,
}

/// Reads smaps into parts: each mapping's header line names it, and its
/// Rss line says how much of it is resident.
fn parts(smaps: &str) -> Parts {
    let mut parts = Parts::default();
    let mut name = "";
    for line in smaps.lines() {
        if let Some(kb) = line.strip_prefix("Rss:") {
            let kb = kb.trim().trim_end_matches("kB").trim().parse().unwrap_or(0);
            let part = if name.contains("JSGCHeap") {
                &mut parts.heap
            } else if name.contains("JITCode") {
                &mut parts.jit
            } else if name.is_empty() || name.starts_with('[') || name.starts_with("/memfd:") {
                &mut parts.anonymous
            } else {
                &mut parts.files
            };
            *part += kb;
        } else if is_header(line) {
            name = line.split_whitespace().nth(5).unwrap_or("");
        }
    }
    parts
}

/// Whether a smaps line opens a mapping, as `start-end perms ...` does.
fn is_header(line: &str) -> bool {
    line.split_whitespace().next().is_some_and(|range| {
        range
            .split_once('-')
            .is_some_and(|(start, end)| is_hex(start) && is_hex(end))
    })
}

fn is_hex(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// What the GPU holds for the shell, in kB: everything its render node's
/// client has, and of that what it shares, which is the buffers niri shows.
#[derive(Debug, Default, PartialEq, Eq)]
struct Gpu {
    total: u64,
    shared: u64,
}

/// Reads a DRM client's fdinfo (the kernel's drm-usage-stats): the totals
/// of every region, `drm-total-<region>` and `drm-shared-<region>`.
fn gpu(fdinfo: &str) -> Gpu {
    let mut gpu = Gpu::default();
    for line in fdinfo.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        // drm-total-cycles-<engine> counts time on an engine, not memory.
        if key.starts_with("drm-total-cycles-") {
            continue;
        }
        let kb = kilobytes(value.trim());
        if key.starts_with("drm-total-") {
            gpu.total += kb;
        } else if key.starts_with("drm-shared-") {
            gpu.shared += kb;
        }
    }
    gpu
}

/// A drm-usage-stats value in kB: a bare count of bytes, or one with KiB or
/// MiB after it.
fn kilobytes(value: &str) -> u64 {
    let mut words = value.split_whitespace();
    let number: u64 = words.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    match words.next() {
        Some("KiB") => number,
        Some("MiB") => number * 1024,
        Some("GiB") => number * 1024 * 1024,
        _ => number / 1024,
    }
}

/// A dma-buf's size in bytes, from its fdinfo's `size:` line.
fn dmabuf_size(fdinfo: &str) -> Option<u64> {
    fdinfo
        .lines()
        .find_map(|line| line.strip_prefix("size:"))
        .and_then(|size| size.trim().parse().ok())
}

/// A process in the shell's service: its pid, name and resident kB.
#[derive(Debug, PartialEq, Eq)]
struct Process {
    pid: u32,
    name: String,
    resident: u64,
}

/// Reads a process's name and resident kB from its /proc status.
fn process(pid: u32, status: &str) -> Process {
    let field = |wanted: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(wanted))
            .map_or("", str::trim)
    };
    Process {
        pid,
        name: field("Name:").to_owned(),
        resident: field("VmRSS:")
            .trim_end_matches("kB")
            .trim()
            .parse()
            .unwrap_or(0),
    }
}

/// Whether a process is the shell itself rather than a helper it started:
/// quickshell, or the wrapper nixpkgs runs it as.
fn is_shell(name: &str) -> bool {
    name == "quickshell" || name.starts_with(".quickshell")
}

/// The shell's service's cgroup, from this program's own: both are the
/// account's systemd's, so the account's part of the path is shared.
fn service_cgroup(own: &str) -> Option<PathBuf> {
    let path = own.lines().find_map(|line| line.strip_prefix("0::"))?;
    let account = &path[..path.find(".service/")? + ".service".len()];
    account.contains("user@").then(|| {
        Path::new("/sys/fs/cgroup")
            .join(account.trim_start_matches('/'))
            .join("session.slice/quickshell.service")
    })
}

/// Everything one line says.
struct Weight {
    shell: Process,
    parts: Parts,
    gpu: Gpu,
    shown: Vec<u64>,
    helpers: Vec<Process>,
    charged: u64,
    peak: u64,
}

fn mb(kb: u64) -> u64 {
    (kb + 512) / 1024
}

/// The line the journal gets, every figure in MB.
fn line(weight: &Weight) -> String {
    let parts = &weight.parts;
    let whole = format!(
        "shell weighs {} MB: JS heap {}, JIT {}, other anonymous {}, files {}; GPU {}, {} of it shared with niri",
        mb(weight.shell.resident),
        mb(parts.heap),
        mb(parts.jit),
        mb(parts.anonymous),
        mb(parts.files),
        mb(weight.gpu.total),
        mb(weight.gpu.shared),
    );
    let mut shown_said = String::new();
    if !weight.shown.is_empty() {
        let mut sizes: Vec<String> = Vec::new();
        let mut shown = weight.shown.clone();
        shown.sort_unstable_by(|a, b| b.cmp(a));
        let mut each = shown.iter().peekable();
        while let Some(&size) = each.next() {
            let mut count = 1;
            while each.peek() == Some(&&size) {
                each.next();
                count += 1;
            }
            sizes.push(format!("{count} of {}", size_said(size)));
        }
        shown_said = format!(" ({})", sizes.join(", "));
    }
    // Helpers under 10 MB, such as a cat reading a pipe, are left unsaid.
    let helpers: Vec<String> = weight
        .helpers
        .iter()
        .filter(|helper| helper.resident >= 10 * 1024)
        .map(|helper| format!("{} {}", helper.name, mb(helper.resident)))
        .collect();
    let helpers_said = if helpers.is_empty() {
        String::new()
    } else {
        format!(", with {}", helpers.join(", "))
    };
    format!(
        "{whole}{shown_said}; its service {} MB, at most {}{helpers_said}",
        mb(weight.charged),
        mb(weight.peak)
    )
}

/// A buffer's size in MB, or "under 1".
fn size_said(bytes: u64) -> String {
    let mb = bytes / (1024 * 1024);
    if mb == 0 {
        "under 1 MB".to_owned()
    } else {
        format!("{mb} MB")
    }
}

/// The figures of one file of the kernel's that holds a single number.
fn number(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Weighs the shell in the service whose cgroup this is.
fn weigh(cgroup: &Path) -> Result<Weight, String> {
    let procs = fs::read_to_string(cgroup.join("cgroup.procs"))
        .map_err(|error| format!("can't read {}: {error}", cgroup.display()))?;
    let mut shell = None;
    let mut helpers = Vec::new();
    for pid in procs
        .lines()
        .filter_map(|pid| pid.trim().parse::<u32>().ok())
    {
        let Ok(status) = fs::read_to_string(format!("/proc/{pid}/status")) else {
            continue;
        };
        let found = process(pid, &status);
        if shell.is_none() && is_shell(&found.name) {
            shell = Some(found);
        } else {
            helpers.push(found);
        }
    }
    let shell = shell.ok_or_else(|| format!("no shell runs in {}", cgroup.display()))?;
    let smaps = fs::read_to_string(format!("/proc/{}/smaps", shell.pid))
        .map_err(|error| format!("can't read the shell's smaps: {error}"))?;
    let mut gpu_found = Gpu::default();
    let mut shown = Vec::new();
    let fds = format!("/proc/{}/fd", shell.pid);
    for fd in fs::read_dir(&fds).into_iter().flatten().flatten() {
        let Ok(target) = fs::read_link(fd.path()) else {
            continue;
        };
        let target = target.to_string_lossy();
        let info = || {
            fs::read_to_string(format!(
                "/proc/{}/fdinfo/{}",
                shell.pid,
                fd.file_name().to_string_lossy()
            ))
        };
        if target.starts_with("/dev/dri/render") && gpu_found.total == 0 {
            if let Ok(info) = info() {
                gpu_found = gpu(&info);
            }
        } else if target.starts_with("/dmabuf:")
            && let Some(size) = info().ok().as_deref().and_then(dmabuf_size)
        {
            shown.push(size);
        }
    }
    Ok(Weight {
        parts: parts(&smaps),
        shell,
        gpu: gpu_found,
        shown,
        helpers,
        charged: number(&cgroup.join("memory.current")) / 1024,
        peak: number(&cgroup.join("memory.peak")) / 1024,
    })
}

fn main() -> ExitCode {
    let cgroup = match env::args_os().nth(1) {
        Some(path) => Some(PathBuf::from(path)),
        None => fs::read_to_string("/proc/self/cgroup")
            .ok()
            .as_deref()
            .and_then(service_cgroup),
    };
    let Some(cgroup) = cgroup else {
        eprintln!("shell-weighs: can't tell where the shell's service is; give its cgroup");
        return ExitCode::FAILURE;
    };
    match weigh(&cgroup) {
        Ok(weight) => {
            println!("{}", line(&weight));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("shell-weighs: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
