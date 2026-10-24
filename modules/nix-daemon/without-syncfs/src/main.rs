//! Runs a program refused `syncfs(2)`: the call answers `ENOSYS`, as
//! though the kernel lacked it, and every other call is left as it was.
//!
//! A VM test's guest syncs the folders it shares with the host, and
//! virtiofsd forwards each as `syncfs()` of the host's whole filesystem,
//! read-only shares too. On btrfs each one commits a transaction, which
//! every program's saves then wait behind. nix-daemon refuses the call to
//! its builds (`modules/nix-daemon/nixos.nix`), but a test a person or an
//! agent runs straight from its driver runs outside the daemon, so core's
//! VM tests start virtiofsd through this instead, wherever they run.
//! Answered `ENOSYS`, the guest's FUSE stops asking, and a test's files
//! are thrown away anyway.
//!
//! Usage: `without-syncfs PROGRAM [ARGUMENT]...`
//!
//! The refusal is a seccomp filter, which needs no privilege once the
//! program, and everything it starts, is barred from gaining any
//! (`PR_SET_NO_NEW_PRIVS`): virtiofsd, run as the test's own user, gains
//! none. Only this machine's own calling convention is filtered, the one
//! virtiofsd is built for, x32's numbers within it included. When the
//! filter can't be put in place, the program runs anyway, and this says
//! so, since a test that runs beats one that fails over the disk's wear.

use std::env;
use std::ffi::{c_int, c_ulong};
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

/// One instruction of a classic BPF program, laid out as the kernel reads
/// one (`struct sock_filter`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Instruction {
    code: u16,
    jump_if_true: u8,
    jump_if_false: u8,
    operand: u32,
}

/// A BPF program as the kernel is handed one (`struct sock_fprog`).
#[repr(C)]
struct Program {
    length: u16,
    instructions: *const Instruction,
}

/// Loads the word at an offset of the call's description.
const LOAD_WORD: u16 = 0x20;
/// Jumps by whether the word loaded equals the operand.
const JUMP_IF_EQUAL: u16 = 0x15;
/// Ends the program with the operand as its answer.
const RETURN: u16 = 0x06;
/// Keeps only the loaded word's bits the operand has.
const AND: u16 = 0x54;

/// Where `struct seccomp_data` holds the call's number, and the calling
/// convention it was made by.
const NUMBER_AT: u32 = 0;
const CONVENTION_AT: u32 = 4;

/// The answers a filter gives: let the call through, or answer it with
/// the errno in its low bits.
const ALLOW: u32 = 0x7fff_0000;
const REFUSE: u32 = 0x0005_0000;
const ENOSYS: u32 = 38;

#[cfg(target_arch = "x86_64")]
const CONVENTION: u32 = 0xc000_003e;
#[cfg(target_arch = "x86_64")]
const SYNCFS: u32 = 306;
/// x32's calls share `x86_64`'s convention, numbered with this bit set, so
/// it's cleared before the number is compared.
#[cfg(target_arch = "x86_64")]
const NUMBER: u32 = !0x4000_0000;
#[cfg(target_arch = "aarch64")]
const CONVENTION: u32 = 0xc000_00b7;
#[cfg(target_arch = "aarch64")]
const SYNCFS: u32 = 267;
#[cfg(target_arch = "aarch64")]
const NUMBER: u32 = !0;

/// Lets every call through but `syncfs`, which it answers `ENOSYS`.
const FILTER: [Instruction; 7] = [
    instruction(LOAD_WORD, 0, 0, CONVENTION_AT),
    // Another calling convention's numbers mean other calls.
    instruction(JUMP_IF_EQUAL, 0, 4, CONVENTION),
    instruction(LOAD_WORD, 0, 0, NUMBER_AT),
    instruction(AND, 0, 0, NUMBER),
    instruction(JUMP_IF_EQUAL, 0, 1, SYNCFS),
    instruction(RETURN, 0, 0, REFUSE | ENOSYS),
    instruction(RETURN, 0, 0, ALLOW),
];

const fn instruction(code: u16, jump_if_true: u8, jump_if_false: u8, operand: u32) -> Instruction {
    Instruction {
        code,
        jump_if_true,
        jump_if_false,
        operand,
    }
}

unsafe extern "C" {
    fn prctl(option: c_int, ...) -> c_int;
}

const PR_SET_NO_NEW_PRIVS: c_int = 38;
const PR_SET_SECCOMP: c_int = 22;
const SECCOMP_MODE_FILTER: c_ulong = 2;

/// Refuses this process, and whatever it runs from now on, `syncfs`.
fn refuse_syncfs() -> io::Result<()> {
    let program = Program {
        length: u16::try_from(FILTER.len()).expect("the filter is seven instructions"),
        instructions: FILTER.as_ptr(),
    };
    let (on, unused): (c_ulong, c_ulong) = (1, 0);
    // SAFETY: prctl reads only the integers it is given here.
    if unsafe { prctl(PR_SET_NO_NEW_PRIVS, on, unused, unused, unused) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel copies the program before prctl returns, and
    // `program` and the `FILTER` it points at outlive the call.
    if unsafe { prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &raw const program) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// What a shell answers for a program it can't run.
const NOT_RUN: u8 = 127;
/// What this answers when it isn't told what to run.
const MISUSED: u8 = 2;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let Some(program) = arguments.next() else {
        eprintln!("Usage: without-syncfs PROGRAM [ARGUMENT]...");
        return ExitCode::from(MISUSED);
    };
    if let Err(error) = refuse_syncfs() {
        eprintln!("without-syncfs: runs without refusing syncfs, since {error}.");
    }
    let error = Command::new(&program).args(arguments).exec();
    eprintln!(
        "without-syncfs: can't run {}: {error}.",
        program.to_string_lossy()
    );
    ExitCode::from(NOT_RUN)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the kernel would answer a call by `FILTER`, run as classic BPF
    /// runs its few instructions here.
    fn answer(convention: u32, number: u32) -> u32 {
        let mut at = 0;
        let mut word = 0;
        loop {
            let step = FILTER[at];
            match step.code {
                LOAD_WORD => {
                    word = match step.operand {
                        CONVENTION_AT => convention,
                        NUMBER_AT => number,
                        other => panic!("loads an unknown offset {other}"),
                    };
                    at += 1;
                }
                AND => {
                    word &= step.operand;
                    at += 1;
                }
                JUMP_IF_EQUAL => {
                    let by = if word == step.operand {
                        step.jump_if_true
                    } else {
                        step.jump_if_false
                    };
                    at += 1 + usize::from(by);
                }
                RETURN => return step.operand,
                other => panic!("runs an unknown instruction {other:#x}"),
            }
        }
    }

    #[test]
    fn answers_syncfs_as_not_implemented() {
        assert_eq!(answer(CONVENTION, SYNCFS), REFUSE | ENOSYS);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn answers_x32s_syncfs_as_not_implemented() {
        assert_eq!(answer(CONVENTION, 0x4000_0000 | SYNCFS), REFUSE | ENOSYS);
    }

    #[test]
    fn lets_every_other_call_through() {
        for number in (0..SYNCFS).chain(SYNCFS + 1..512) {
            assert_eq!(answer(CONVENTION, number), ALLOW, "call {number}");
        }
    }

    #[test]
    fn lets_another_conventions_calls_through() {
        // i386's numbers, where SYNCFS names another call.
        let i386 = 0x4000_0003;
        assert_eq!(answer(i386, SYNCFS), ALLOW);
    }
}
