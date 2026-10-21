# The Guide

A Rust program that walks a person through a machine's whole life: making
the stick that installs it, installing it with its disk encrypted and Secure
Boot on, sealing it to the TPM with YubiKeys as the way back in, and every
repair after. It's for someone who won't remember any of it, so it says
everything it needs before it starts and never leaves them guessing what
happened. This crate holds the step engine, what the guide observes, and
the stick's writer and vault; the steps and the screens come later.

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
- **The stick.** The image's partitions are copied byte for byte and its
  partition entries and disk GUID kept, since the stick's system finds its
  store by them. A vault partition fills the rest, and both tables are
  written for the stick's size. Both are cleared first and written last, so
  a write that stops part way leaves no table rather than one pointing at
  half a stick. Everything reaches the stick in aligned 64 KiB blocks, as a
  Mac's raw disks need. Reading back forgets the system's cached copy first,
  then compares both tables, hashes each partition again and opens the
  vault.
- **The vault.** A FAT32 partition any machine can read, holding files
  sealed with age to a vault key made for it. The key is sealed to each
  YubiKey, and separately to the recovery passphrase, since age won't mix a
  passphrase with other recipients. Files are split into objects of at most
  1 GiB, under FAT32's limit, with numbered names; only the sealed index
  says what they hold, and extracting checks every object's size and each
  file's SHA-256. `manifest.json` goes last, so a vault without one was
  never finished. Every file sits at the top, since fatfs 0.3.6 writes
  directories fsck.fat has to repair.

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
| `src/stick/`          | writing the stick and reading it back: its tables, and aligned blocks |
| `src/vault/`          | the vault's sealing and index, and where its files are kept |
| `fixtures/<hardware>/` | what each tool printed on a real machine, as it printed it |
| `tests/journey.rs`    | the journey's promises, with toy steps                    |
| `tests/stick.rs`      | whole sticks written into files, judged by sfdisk and fsck.fat |

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt. `cargo test` runs in a second, and the package runs the
tests again as it builds, then `guide tools`. `nix fmt` from the repository
root formats the Rust. `guide observe` prints what the guide sees on the
machine it runs on and changes nothing, which is how a parser meets a
machine it hasn't seen. `guide write-stick IMAGE FILE` writes a repair stick
into a file already the size of a stick, and refuses a disk.

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
