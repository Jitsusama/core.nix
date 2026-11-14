//! The reading of each kernel file, over excerpts of the files Joel's shell
//! gave on 2026-11-11, and the line said from them.

use super::*;

const SMAPS: &str = "\
5ecac5ab3000-5ecac5ab4000 r--p 00000000 00:1f 123 /nix/store/x-quickshell/bin/.quickshell-wrapped
Size:                  4 kB
Rss:                   4 kB
Pss:                   4 kB
7f0000000000-7f0010000000 rw-p 00000000 00:01 9 /memfd:JSGCHeap:QtQml (deleted)
Size:             262144 kB
Rss:              208344 kB
7f0010000000-7f0010100000 r-xp 00000000 00:01 10 /memfd:JITCode:QtQml (deleted)
Size:               1024 kB
Rss:                 812 kB
7f0020000000-7f0024000000 rw-p 00000000 00:00 0
Size:              65536 kB
Rss:               61000 kB
7f0030000000-7f0030100000 rw-p 00000000 00:00 0                          [heap]
Size:               1024 kB
Rss:                1000 kB
7f0040000000-7f0040100000 rw-s 00000000 00:01 11 /memfd:unknown-usage:QtQml (deleted)
Size:                368 kB
Rss:                 368 kB
7f0050000000-7f0050800000 r--p 00000000 00:1f 12 /nix/store/y-llvm/lib/libLLVM.so.21.1
Size:              81920 kB
Rss:               25592 kB
7ffd00000000-7ffd00021000 rw-p 00000000 00:00 0                          [stack]
Size:                132 kB
Rss:                  60 kB
";

#[test]
fn the_heap_and_the_jits_code_are_told_apart_from_other_anonymous_memory() {
    assert_eq!(
        parts(SMAPS),
        Parts {
            heap: 208_344,
            jit: 812,
            anonymous: 61_000 + 1000 + 368 + 60,
            files: 4 + 25_592,
        }
    );
}

#[test]
fn a_line_inside_a_mapping_never_opens_another() {
    // A VmFlags line starts with letters that are hex digits, but has no
    // range of addresses.
    assert!(
        !is_header("VmFlags: rd wr mr mw me ac sd"),
        "a VmFlags line was taken for a mapping's header"
    );
    assert!(
        is_header("7f0000000000-7f0010000000 rw-p 00000000 00:01 9 /x"),
        "a mapping's header was not taken for one"
    );
}

const RENDER_NODE: &str = "\
pos:\t0
flags:\t02100002
drm-driver:\txe
drm-client-id:\t298
drm-total-system:\t4168 KiB
drm-shared-system:\t0
drm-resident-system:\t0
drm-total-gtt:\t259932 KiB
drm-shared-gtt:\t83360 KiB
drm-active-gtt:\t0
drm-resident-gtt:\t259932 KiB
drm-total-stolen:\t0
drm-total-cycles-rcs:\t426224886365
";

#[test]
fn the_gpu_counts_every_region_once_and_not_its_cycles() {
    assert_eq!(
        gpu(RENDER_NODE),
        Gpu {
            total: 4168 + 259_932,
            shared: 83_360,
        }
    );
}

#[test]
fn a_count_without_a_unit_is_bytes() {
    assert_eq!(kilobytes("2048"), 2);
    assert_eq!(kilobytes("3 MiB"), 3072);
}

#[test]
fn a_dma_bufs_size_is_read_from_its_fdinfo() {
    let info = "pos:\t0\nflags:\t02000002\nmnt_id:\t16\nino:\t1063\nsize:\t33554432\ncount:\t2\nexp_name:\tdrm\n";
    assert_eq!(dmabuf_size(info), Some(33_554_432));
    assert_eq!(dmabuf_size("pos:\t0\n"), None);
}

#[test]
fn a_process_is_read_from_its_status() {
    let status = "Name:\t.quickshell-wra\nState:\tS (sleeping)\nVmRSS:\t  651220 kB\n";
    assert_eq!(
        process(2_936_825, status),
        Process {
            pid: 2_936_825,
            name: ".quickshell-wra".to_owned(),
            resident: 651_220,
        }
    );
    assert!(
        is_shell(".quickshell-wra"),
        "quickshell's wrapped name was not taken for the shell"
    );
    assert!(!is_shell("pi"), "pi was taken for the shell");
}

#[test]
fn the_shells_service_is_found_beside_this_programs_own() {
    let own = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/shell-weighs.service\n";
    assert_eq!(
        service_cgroup(own),
        Some(PathBuf::from(
            "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service"
        ))
    );
    assert_eq!(service_cgroup("0::/system.slice/sshd.service\n"), None);
}

/// A shell weighed on 2026-11-11, its figures as they were.
fn weighed() -> Weight {
    Weight {
        shell: Process {
            pid: 2_936_825,
            name: ".quickshell-wra".to_owned(),
            resident: 651 * 1024,
        },
        parts: Parts {
            heap: 200 * 1024,
            jit: 5 * 1024,
            anonymous: 329 * 1024,
            files: 115 * 1024,
        },
        gpu: Gpu {
            total: 253 * 1024,
            shared: 81 * 1024,
        },
        shown: vec![
            491_520, 33_554_432, 8_388_608, 33_554_432, 491_520, 8_388_608, 491_520,
        ],
        helpers: vec![
            Process {
                pid: 2,
                name: "pi".to_owned(),
                resident: 87 * 1024,
            },
            Process {
                pid: 3,
                name: "cat".to_owned(),
                resident: 600,
            },
        ],
        charged: 986 * 1024,
        peak: 1183 * 1024,
    }
}

#[test]
fn the_line_says_each_part_and_what_the_service_holds_besides() {
    assert_eq!(
        line(&weighed()),
        "shell weighs 651 MB: JS heap 200, JIT 5, other anonymous 329, files 115; GPU 253, \
         81 of it shared with niri (2 of 32 MB, 2 of 8 MB, 3 of under 1 MB); its service \
         986 MB, at most 1183, with pi 87"
    );
}

#[test]
fn a_script_gets_every_figure_in_kb_unrounded() {
    let answer = answer(&weighed());
    assert_eq!(answer["schema"], "shell-weighs.weight/1");
    assert_eq!(answer["shell"]["resident_kb"], 651 * 1024);
    assert_eq!(answer["parts_kb"]["heap"], 200 * 1024);
    assert_eq!(answer["gpu_kb"]["shared"], 81 * 1024);
    assert_eq!(answer["shown_bytes"][1], 33_554_432);
    // Every helper is there, the small ones the line leaves unsaid too.
    assert_eq!(answer["helpers"][1]["name"], "cat");
    assert_eq!(answer["peak_kb"], 1183 * 1024);
}

#[test]
fn a_cgroup_with_no_service_is_not_found() {
    let missing = std::env::temp_dir().join("shell-weighs-no-such-service");
    let failure = weigh(&missing)
        .err()
        .expect("a missing cgroup weighs nothing");
    assert_eq!(failure.exit, Exit::NotFound, "{failure:?}");
}
