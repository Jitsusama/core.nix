# The Guide

A Rust program that walks a person through a machine's whole life: making
the stick that installs it, installing it with its disk encrypted and Secure
Boot on, sealing it to the TPM with YubiKeys as the way back in, and every
repair after. It's for someone who won't remember any of it, so it says
everything it needs before it starts and never leaves them guessing what
happened. This crate holds the step engine and what the guide observes; the
steps, the screens and the stick's writer come later.

## How It Decides

- **Places.** A journey runs on the machine making the stick, then on the
  stick booted on the machine, then on the installed machine, and never goes
  back. The guide only looks at steps for the place it's running in, since it
  can't observe the others from there.
- **Observing.** A step says whether it's done by looking at the machine:
  the firmware's variables, sbctl's keys, the boot loader's entries, the
  TPM's lockout, the disks, the FIDO2 keys plugged in. What comes next is the
  first step here that isn't done, worked out afresh every time, so the guide
  starting again after a reboot or a crash carries on where it left off.
- **Proving.** A step is done only when observing it again after it runs says
  so. A step whose work succeeded but left the machine unchanged is a fault,
  never progress. A step already done is never run again.
- **Asking.** A step names everything it needs from the person (a PIN, a key
  plugged in, a disk's name typed back) before it starts, and the journey
  refuses to run it until each is answered. A secret is wiped from memory
  when dropped, and Debug never shows it.
- **Journal.** What observing can't show, such as a recovery key proven by
  being typed back, goes in the journal: a line of JSON per fact, on the disk
  before the step goes on, readable only by its owner.
- **Tools.** Every program runs by the store path `package.nix` gave it, with
  an empty environment but for `LC_ALL=C.UTF-8`, so no PATH can swap one in
  and what it prints is what the parsers were written against. Every call is
  logged raw. tpm2-tools always names `/dev/tpmrm0`, since otherwise it tries
  a D-Bus broker first.

## Where Things Live

| Path                  | Holds                                                     |
| --------------------- | --------------------------------------------------------- |
| `src/step.rs`         | a step, where it stands, and what it asks for             |
| `src/journey.rs`      | the order, what's next, and running a step to its proof   |
| `src/place.rs`        | where the guide runs                                      |
| `src/machine.rs`      | what a step observes and acts through                     |
| `src/system.rs`       | the real machine: tools by store path, and the log        |
| `src/fake.rs`         | the machine tests give steps, which answers only what it's told |
| `src/tool.rs`         | every program the guide runs, and where this build finds it |
| `src/journal.rs`      | the facts observing can't show                            |
| `src/fault.rs`        | what stops a step, in a person's words                    |
| `src/observe/`        | one parser per thing the guide reads off a machine        |
| `fixtures/<hardware>/` | what each tool printed on a real machine, as it printed it |
| `tests/journey.rs`    | the journey's promises, with toy steps                    |

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt. `cargo test` runs in a second, and the package runs the
tests again as it builds, then `guide tools`. `nix fmt` from the repository
root formats the Rust. `guide observe` prints what the guide sees on the
machine it runs on and changes nothing, which is how a parser meets a
machine it hasn't seen.

- Parse what a tool printed on a real machine, never what its manual says it
  prints: capture it into `fixtures/<hardware>/`, named as core's hardware
  modules are, record the command in `fixtures/README.md`, and read it with
  the `xps!` or `surface!` macro. A firmware or tool that differs (the
  Surface's firmware has no `AuditMode` or `DeployedMode`) gets its own
  fixture.
- Replace what names a person's machine rather than its hardware, such as a
  host name or a drive's serial, and say so in `fixtures/README.md`.
- Write a test first and see it fail on its assertion before making it pass.
  Before trusting a test of a rule already written, break the rule and watch
  the test fail.
- A test never reaches the machine. Steps take `&mut dyn Machine`, and tests
  give them `Fake`, which refuses any call or file its test didn't script.
- A new program goes in `src/tool.rs` and `package.nix` together, and
  `guide tools` fails the build until it has a path.
