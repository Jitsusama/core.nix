//! The kernel's side of the guard: a cgroup device program that refuses
//! every video4linux character device and lets every other through, and
//! the bpf(2) calls that load it, put it on a cgroup, find it there and
//! take it off again.
//!
//! The kernel asks a cgroup's device programs, and those of every cgroup
//! above it, as a process in it opens a device node, so one program on an
//! agents' slice reaches every process under it, those started later
//! included. It's put on beside any program systemd keeps there
//! (`BPF_F_ALLOW_MULTI`), and the kernel lets the open through only when
//! all of them do. A camera already open stays open: the program is asked
//! only at the open.
//!
//! It's found again by its name rather than by a file descriptor kept
//! here, so a guard started again after a crash, or by a switch, takes
//! off the program an earlier run left, and never puts on a second.

use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;

/// The calls of bpf(2) used, from linux/bpf.h's `enum bpf_cmd`.
const PROG_LOAD: libc::c_long = 5;
const PROG_ATTACH: libc::c_long = 8;
const PROG_DETACH: libc::c_long = 9;
const PROG_GET_FD_BY_ID: libc::c_long = 13;
const OBJ_GET_INFO_BY_FD: libc::c_long = 15;
const PROG_QUERY: libc::c_long = 16;
/// `BPF_PROG_TYPE_CGROUP_DEVICE` and `BPF_CGROUP_DEVICE`.
const CGROUP_DEVICE_PROGRAM: u32 = 15;
const CGROUP_DEVICE_ATTACH: u32 = 6;
/// `BPF_F_ALLOW_MULTI`: beside the programs already there.
const ALLOW_MULTI: u32 = 2;
/// The size of the attributes handed to every call: past the fields any of
/// them reads, and zero beyond, as the kernel requires.
const ATTR: usize = 128;
/// The most programs a cgroup is asked for at once.
const MOST: usize = 64;
/// Where `bpf_prog_info` holds a program's name, and the name's length.
const INFO_NAME: usize = 64;
const NAME_LENGTH: usize = 16;

/// The program's name, by which a run finds what an earlier one put on.
const NAME: &str = "camera_guard";

/// linux/bpf.h's `BPF_DEVCG_DEV_CHAR`, the type in the low half of the
/// device's access word, and video4linux's major number, every camera's.
const CHARACTER: i32 = 2;
const VIDEO4LINUX: i32 = 81;

/// The program, one instruction a line: its opcode, its registers (the
/// destination low, the source high), its offset and its immediate. The
/// context is `struct bpf_cgroup_dev_ctx`: the access word, then the major
/// and minor numbers. It returns 0 to refuse and 1 to allow.
const PROGRAM: [(u8, u8, i16, i32); 9] = [
    // r2 = the access word; r2 &= 0xffff, its device type.
    (0x61, 0x12, 0, 0),
    (0x57, 0x02, 0, 0xffff),
    // Not a character device: allowed.
    (0x55, 0x02, 4, CHARACTER),
    // r2 = the major number; not video4linux: allowed.
    (0x61, 0x12, 4, 0),
    (0x55, 0x02, 2, VIDEO4LINUX),
    // A camera: refused.
    (0xb7, 0x00, 0, 0),
    (0x95, 0x00, 0, 0),
    (0xb7, 0x00, 0, 1),
    (0x95, 0x00, 0, 0),
];

/// The program as the kernel reads it, eight bytes an instruction.
fn instructions() -> Vec<u8> {
    PROGRAM
        .iter()
        .flat_map(|&(code, registers, offset, immediate)| {
            let mut instruction = vec![code, registers];
            instruction.extend(offset.to_le_bytes());
            instruction.extend(immediate.to_le_bytes());
            instruction
        })
        .collect()
}

fn put32(attr: &mut [u8; ATTR], at: usize, value: u32) {
    attr[at..at + 4].copy_from_slice(&value.to_ne_bytes());
}

fn put64(attr: &mut [u8; ATTR], at: usize, value: u64) {
    attr[at..at + 8].copy_from_slice(&value.to_ne_bytes());
}

fn fd32(fd: &impl AsRawFd) -> u32 {
    u32::try_from(fd.as_raw_fd()).expect("an open descriptor is never negative")
}

fn address<T>(pointer: *const T) -> u64 {
    pointer as usize as u64
}

/// Calls bpf(2) with `attr`, answering what it returns.
fn bpf(command: libc::c_long, attr: &mut [u8; ATTR]) -> io::Result<libc::c_long> {
    // SAFETY: attr is ATTR bytes the kernel may read and write, and every
    // address in it points at a buffer that outlives the call, of the
    // length given beside it.
    let said = unsafe { libc::syscall(libc::SYS_bpf, command, attr.as_mut_ptr(), ATTR) };
    if said < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(said)
    }
}

fn descriptor(said: libc::c_long) -> OwnedFd {
    let fd = i32::try_from(said).expect("a descriptor fits an int");
    // SAFETY: the kernel just gave this descriptor, and nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
}

/// The guard's program, loaded.
pub struct Guard {
    program: OwnedFd,
}

impl Guard {
    /// Loads the program.
    ///
    /// # Errors
    ///
    /// The kernel refuses it, with what its verifier said.
    pub fn load() -> io::Result<Self> {
        let code = instructions();
        let license = b"MIT\0";
        let mut log = vec![0_u8; 64 * 1024];
        let mut attr = [0_u8; ATTR];
        put32(&mut attr, 0, CGROUP_DEVICE_PROGRAM);
        put32(
            &mut attr,
            4,
            u32::try_from(PROGRAM.len()).expect("nine fit"),
        );
        put64(&mut attr, 8, address(code.as_ptr()));
        put64(&mut attr, 16, address(license.as_ptr()));
        put32(&mut attr, 24, 1);
        put32(
            &mut attr,
            28,
            u32::try_from(log.len()).expect("64 KiB fits"),
        );
        put64(&mut attr, 32, address(log.as_mut_ptr()));
        attr[48..48 + NAME.len()].copy_from_slice(NAME.as_bytes());
        let said = bpf(PROG_LOAD, &mut attr).map_err(|error| {
            let end = log.iter().position(|&byte| byte == 0).unwrap_or(log.len());
            let verifier = String::from_utf8_lossy(&log[..end]);
            io::Error::new(error.kind(), format!("{error}: {}", verifier.trim()))
        })?;
        Ok(Self {
            program: descriptor(said),
        })
    }

    /// Puts the program on the cgroup at `path`, or takes it off, as `on`
    /// says, answering whether anything changed.
    ///
    /// # Errors
    ///
    /// The cgroup isn't there, or the kernel refuses.
    pub fn set(&self, path: &Path, on: bool) -> io::Result<bool> {
        let cgroup = File::open(path)?;
        let ours = ours(&cgroup)?;
        if on && ours.is_empty() {
            let mut attr = [0_u8; ATTR];
            put32(&mut attr, 0, fd32(&cgroup));
            put32(&mut attr, 4, fd32(&self.program));
            put32(&mut attr, 8, CGROUP_DEVICE_ATTACH);
            put32(&mut attr, 12, ALLOW_MULTI);
            bpf(PROG_ATTACH, &mut attr)?;
            return Ok(true);
        }
        if on {
            return Ok(false);
        }
        for program in &ours {
            let mut attr = [0_u8; ATTR];
            put32(&mut attr, 0, fd32(&cgroup));
            put32(&mut attr, 4, fd32(program));
            put32(&mut attr, 8, CGROUP_DEVICE_ATTACH);
            bpf(PROG_DETACH, &mut attr)?;
        }
        Ok(!ours.is_empty())
    }
}

/// The programs named as the guard's on `cgroup` itself, not those above.
fn ours(cgroup: &File) -> io::Result<Vec<OwnedFd>> {
    let mut ids = [0_u32; MOST];
    let mut attr = [0_u8; ATTR];
    put32(&mut attr, 0, fd32(cgroup));
    put32(&mut attr, 4, CGROUP_DEVICE_ATTACH);
    put64(&mut attr, 16, address(ids.as_mut_ptr()));
    put32(&mut attr, 24, u32::try_from(MOST).expect("64 fits"));
    bpf(PROG_QUERY, &mut attr)?;
    let count = u32::from_ne_bytes(attr[24..28].try_into().expect("four bytes"));
    let mut found = Vec::new();
    for &id in ids
        .iter()
        .take(usize::try_from(count).expect("a count fits"))
    {
        let mut attr = [0_u8; ATTR];
        put32(&mut attr, 0, id);
        // A program taken off since the query is gone, and isn't ours to
        // take off.
        let Ok(said) = bpf(PROG_GET_FD_BY_ID, &mut attr) else {
            continue;
        };
        let program = descriptor(said);
        if named(&program)? == NAME {
            found.push(program);
        }
    }
    Ok(found)
}

/// The name the kernel holds for `program`.
fn named(program: &OwnedFd) -> io::Result<String> {
    let mut info = [0_u8; INFO_NAME + NAME_LENGTH];
    let mut attr = [0_u8; ATTR];
    put32(&mut attr, 0, fd32(program));
    put32(&mut attr, 4, u32::try_from(info.len()).expect("80 fits"));
    put64(&mut attr, 8, address(info.as_mut_ptr()));
    bpf(OBJ_GET_INFO_BY_FD, &mut attr)?;
    let name = &info[INFO_NAME..];
    let end = name
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(NAME_LENGTH);
    Ok(String::from_utf8_lossy(&name[..end]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the program answers for a device, worked out from its
    /// instructions as the kernel would run them, for the tests to hold.
    fn allows(kind: i32, major: i32) -> bool {
        let context = [kind, major, 0];
        let mut registers = [0_i64; 11];
        let mut at = 0_usize;
        loop {
            let (code, regs, offset, immediate) = PROGRAM[at];
            let destination = usize::from(regs & 0x0f);
            at += 1;
            match code {
                0x61 => {
                    let index = usize::try_from(offset / 4).expect("a context offset is positive");
                    registers[destination] = i64::from(context[index]);
                }
                0x57 => registers[destination] &= i64::from(immediate),
                0x55 if registers[destination] != i64::from(immediate) => {
                    at += usize::try_from(offset).expect("a jump here goes forward");
                }
                0x55 => {}
                0xb7 => registers[destination] = i64::from(immediate),
                0x95 => return registers[0] == 1,
                other => unreachable!("the program holds no opcode {other:#x}"),
            }
        }
    }

    #[test]
    fn the_program_refuses_a_camera_and_nothing_else() {
        assert!(
            !allows(CHARACTER, VIDEO4LINUX),
            "a camera's node was let through"
        );
        for (kind, major, what) in [
            (CHARACTER, 1, "/dev/null's"),
            (CHARACTER, 136, "a terminal's"),
            (CHARACTER, 116, "a sound card's"),
            (CHARACTER, 226, "the GPU's"),
            (1, VIDEO4LINUX, "a block device with video4linux's number"),
        ] {
            assert!(allows(kind, major), "{what} node was refused");
        }
    }

    #[test]
    fn an_access_word_s_upper_half_doesn_t_change_the_answer() {
        // The kernel puts read, write and mknod in the word's upper half.
        for access in [1, 2, 4, 7] {
            assert!(
                !allows((access << 16) | CHARACTER, VIDEO4LINUX),
                "a camera opened for access {access} was let through"
            );
            assert!(
                allows((access << 16) | CHARACTER, 1),
                "a device opened for access {access} was refused"
            );
        }
    }

    #[test]
    fn the_program_is_nine_eight_byte_instructions_and_its_name_fits() {
        assert_eq!(
            instructions().len(),
            9 * 8,
            "the program isn't laid out as the kernel reads it"
        );
        assert!(
            NAME.len() < NAME_LENGTH,
            "the program's name is past what the kernel keeps"
        );
        assert!(
            NAME.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
            "the program's name holds a character the kernel refuses"
        );
    }
}
