# The Guide

A Rust program that walks a person through a machine's whole life: making
the stick that installs it, installing it with its disk encrypted and Secure
Boot on, sealing it to the TPM with YubiKeys as the way back in, and every
repair after. It's for someone who won't remember any of it, so it says
everything it needs before it starts and never leaves them guessing what
happened. This crate holds the step engine, what the guide observes, the
stick's writer and vault, the install from the stick and the screen it's
driven on; the installed machine's steps come later.

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
- **Kept secrets.** A secret the person has to keep (the disk's recovery
  key, a stick's passphrase, a rescue bundle's) is seven words the guide
  makes from EFF's large list, about 90 bits, never one they invent. The
  steps for keeping it in 1Password on their iPhone come first, then the
  words alone on the screen, so the Camera's Live Text copies nothing
  else, then the words typed back without being shown. Typed words are
  read as the words they say, whatever their case or spacing, wherever
  they're typed.
- **Journal.** What observing can't show, such as a recovery key proven by
  being typed back, goes in the journal: a line of JSON per fact, on the disk
  before the step goes on, readable only by its owner.
- **Secrets.** None is ever an argument. A program is given each the way it
  reads it: an environment variable (`PIN`, `PASSWORD`), a credential file
  under /run, which is memory, made for that one call
  (`cryptenroll.fido2-pin`), or standard input. The log names where each
  went and never what it was, and leaves out what a program prints when
  that's a secret too, such as a recovery key.
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
- **Signing.** A stick for one machine has its boot program,
  `EFI/BOOT/BOOTX64.EFI` and nothing else on its ESP, signed with that
  machine's db key, so it boots there with Secure Boot enforced. The
  signature is byte for byte what sbsign makes from the same key at the same
  moment, since sbsign's is what every machine here boots; sbsign itself is
  Linux's alone. The ESP is signed in memory before anything reaches the
  stick, so a key that won't sign leaves the stick as it was. A program
  signed already, its signature at its end as sbsign leaves it, is signed
  again in place of that signature, which at the same moment is exactly
  what signing it the first time made; a signature anywhere else is
  refused. Keys are sbctl's file keys, a `db.key` and `db.pem` in one
  directory.
- **The host.** A stick is written on whatever machine writes it, as the
  person and never root. Only a whole USB or removable disk counts, and
  only one may be plugged in; what's mounted from it is unmounted, and its
  disk is opened through the system's own way of letting a person write
  one: UDisks2, which asks polkit, on Linux, and authopen on a Mac. A
  machine's repository offers `nix run .#guide`, core's `lib.guide` with
  that machine's image and payload built in. The last card says how to
  reach that machine's firmware, as its hardware module says in
  `jitsusama.firmware.startFromStick` and `clearSecureBootKeys`: plugging
  the stick in with the machine off, and clearing its keys first only
  when the stick carries none, switched off as the firmware's setup is
  left, so a system already on its disk never starts and puts its keys
  back.
- **The install.** On the stick, as root, from what the vault holds for the
  machine (`payload.nix`): its target, its disk's layout and its system as
  a binary cache. Before the disk is touched, the firmware is checked to
  take the keys the install will give the machine, so a machine is never
  wiped only to be unbootable. The disk is wiped only once the machine's
  name is typed back for it, then laid out by the machine's own disko
  script, run with the stick's disk tools, its volume made with a key held
  only in memory.
  Each YubiKey is enrolled alone, told apart from the others by a silent
  assertion, and proven by opening the disk. The Secure Boot keys come
  next, then a recovery key, its words held before they're enrolled so a
  rerun enrols no second slot, enrolled with the volume's own key
  derivation and marked as systemd marks a recovery key. It seals the
  machine's rescue bundle while it's still held, before it's kept on the
  phone, typed back and proven, so no
  machine leaves its install without one. The Secure Boot keys are put
  back from the vault when the stick carries the ones the firmware's db
  already trusts and the firmware has a platform key, so a reinstall
  needs no firmware visit, and otherwise made where the install signs
  with them, which only a firmware in setup mode will take. In setup mode
  they're always made, whatever the stick carries, since the firmware
  would enrol any keys it's given. The system is copied in from the vault with no
  network, the account given its password and the key held in memory
  taken off the disk, and the stick is made the machine's own (below)
  last, since it needs none of those and a stick that won't take it
  should cost only the stick. A host gives the stick the keys
  from the machine's bundle, the one it keeps or one `--rescue-bundle`
  names, opened in memory, and signs the stick with them.
  Every step stays done through the steps after it, since the journey runs
  the first that isn't.
- **Recovering.** Any stick recovers a machine with its data left in
  place, since the disk says what it is: core's layout on an internal
  disk, an encrypted partition beside its `<name>-boot`. The disk is shown
  before anything is typed, opened with the recovery key or the plugged-in
  YubiKey's PIN (never the TPM, whose seal a stick's boot doesn't match),
  every subvolume and the ESP mounted under
  /mnt. Then a choice, back after each: a root shell inside it; which
  generation boots next, set in the firmware's `LoaderEntryDefault` as
  `bootctl set-default` would, since from a stick bootctl refuses; or a
  lost YubiKey replaced: the key still had told apart by a silent
  assertion, the new one enrolled and proven with the recovery key, and
  only then the lost one's slots wiped, so the disk never has fewer ways
  in; or the boot loader installed again by the installed system's own
  `switch-to-configuration boot`, which signs with the machine's keys.
  The kernel writes only errors on the stick's console, which the guide
  owns. A repair stick runs this when it boots; `guide recover` runs it
  from any stick's shell. A machine's own stick asks first, when a disk
  already holds a machine and the install hasn't been confirmed onto it
  this boot, whether to recover it or install over it, so one stick does
  both. The other repairs the plan lists are still to come.
- **Choosing.** A choice is a list the arrows move through and Return
  takes, the cursor mark in the accent on the one the keys are on.
- **The installed machine.** Once it boots its own disk, the guide runs
  there from core's `guide` NixOS module, which also writes what it reads
  about the machine to `/etc/guide/machine.json`. The module's
  `guide-installed` unit runs it as root on the first console at every
  boot, before the sign-in, so nothing has to be remembered: it shows
  only when something needs doing, and otherwise leaves at once. It's
  after lanzaboote's unit that restarts the first boot to enrol the keys
  and the one that makes the disk's policy. While it runs, systemd holds
  its lines about the units still starting, which would otherwise land
  across the guide wherever the console is the screen, as on a laptop.
  NixOS's test driver turns those lines off, so the install test turns
  them back on, as a laptop has them. Its last card signs in rather
  than restarting, since a boot it holds back hasn't counted as good and
  a restart would spend one of a new generation's tries. Run by hand, it
  asks for root once through run0, before taking the screen: asked for
  each program, run0 couldn't ask at all, since the guide holds their
  input and output. A step that stops shows Ctrl+C beside Return, since
  at boot it holds the sign-in back. It checks
  Secure Boot enforces with the machine's own keys before anything is
  sealed to it, then seals the disk to the TPM with a PIN under the
  measured-boot policy, a YubiKey opening the disk to do so, prefers the
  TPM's slot and proves it opens. The seal counts as done only while the
  policy follows the boot: the TPM holds its NV index, and lanzaboote's
  unit remade it this boot. A cleared TPM loses the index, and a firmware
  update changes PCR 0 past what the policy can follow, so that unit
  fails. The disk then opens with a YubiKey, and the guide has
  systemd-pcrlock drop the stale policy and the unit make a new one
  before sealing again. A boot through the firmware's own menu fails the
  unit too, since the menu program is measured into PCR 4, and there the
  guide leaves the seal alone: replayed without some of the firmware's
  programs before the separator, the event log gives a PCR 4 the policy
  allows, with every other PCR allowed as it is. Sealing then would
  refuse the next ordinary boot, which opens with the TPM's PIN. Its
  secrets go only as files in memory and on
  standard input, never on a command line.
- **The keys in the TPM.** Once the disk is sealed, the installed guide
  makes each SSH key the accounts keep in the TPM that's missing: every
  one a home-manager module names in `jitsusama.tpmKeys`, which the
  `guide` module gathers into `jitsusama.guide.keys` and machine.json.
  It makes each as its account, through runuser and ssh-tpm-keygen, with
  one PIN typed twice for every key that asks for one. ssh-tpm-keygen
  reads a PIN only from a terminal and makes the key with none through
  pipes, so its `SSH_ASKPASS` is the guide itself, which prints
  `GUIDE_ASKPASS`, set only in the keygen's environment. A key counts as
  made when both its halves are there and the public one is what
  `--print-pubkey` reads from the private one; half a key blocks rather
  than being made over. A cleared TPM leaves the keys unable to load,
  which the guide can't see without their PIN: move them away for it to
  make new ones.
- **The spares on the YubiKeys.** Then the installed guide keeps the
  login key's spare, a resident ed25519-sk key, on each YubiKey the
  machine's rescue bundle names, for the one account with
  `jitsusama.yubikeySpare` (a YubiKey holds one, so the module refuses
  two). It asks for each by name and first downloads what the key
  holds, making a spare only on a key holding none, since a new one
  would strand every place the old one was added. The installed machine
  can't tell its YubiKeys apart by name, so the first key to give a
  spare is remembered under that name by the disk credential it holds,
  and a key holding another name's credential is refused. ssh-keygen
  runs as root in /run/guide/spare, since at boot nobody is signed in
  for the account to reach the key, and the files are the account's
  once moved. Its `SSH_ASKPASS` is the guide, as for the TPM's keys,
  with `-N ""` so the file's own passphrase isn't answered with the PIN.
- **The rescue bundle.** A machine's Secure Boot keys, all seven files
  sbctl keeps, sealed under a key of their own, which is sealed to the
  machine's YubiKeys and apart to a recovery passphrase of seven words,
  as the vault's is. `guide seal-keys` makes it on the machine the keys belong to,
  reading them as root and sealing them in memory, and proves the
  passphrase opens the same keys before anything's written. It's text
  and ciphertext only, so it can live on every machine and stick, and it
  names the db certificate's fingerprint, so a machine knows its bundle
  is current without opening it. The install seals one to the disk's
  recovery key in place of a passphrase, kept at
  `/var/lib/guide/rescue-bundle.json` where anyone can read it. A host
  writing the machine's stick opens it with a YubiKey or what its
  passphrase is, holding the keys only in memory, so plain Secure Boot
  keys are never an input anywhere.
- **Spreading the bundles.** Before a host writes over a stick, it keeps
  every bundle the stick's vault carries, read where a desktop mounted it
  or else mounted only for reading, which needs no password as opening
  the disk would, and its own machine's,
  under `$XDG_STATE_HOME/guide/rescue/`, each written whole before it
  takes an older one's place. The host's own machine's always stays for
  that machine; of two others for one machine the one sealed later stays,
  never one said to be sealed more than a day past the host's clock, for
  at most 32 machines, each named as a hostname is. Every stick it writes
  carries all it keeps beside the vault's sealed files, read back as
  written, and a stick for a machine it keeps a bundle for is signed from
  that bundle unless `--rescue-bundle`, `--sign-with` or
  `--no-rescue-bundle` is given; one that isn't opened says where it is.
  Only the stick gathered from is written over. So a bundle reaches every
  laptop and stick by their being used. A forged one does no harm to a
  machine: an install puts keys back only when the firmware has a
  platform key and its db trusts them already, so a stick signed from one
  only fails to boot. A vault
  that won't be read doesn't stop a stick being written, and the host
  says its bundles weren't kept; failing to keep what was read does.
- **The machine's own stick.** Once the install has the machine's keys,
  it puts them into the stick's vault under `sbctl/`, where a reinstall
  takes them from, and the bundle beside the vault's sealed files as
  `<machine>-rescue-bundle.json`, so the vault's passphrase now guards
  the machine's PK, KEK and db keys. The keys' objects go first, then one
  index naming them, written whole under a name of its own before it
  takes the old index's place, so a stick pulled part way opens as it was
  or as it is now. Then it signs the stick's boot program with the
  machine's db key, in place of whatever signed it, writing back only the
  blocks that changed and reading them back. One the machine's key signed
  already, as a reinstall from its own stick finds it, isn't written to
  at all. Pulled out, the
  stick boots under the machine's own keys and reinstalls it with no
  visit to the firmware. It changes only the partition named `esp` on the
  disk its vault is on, as lsblk sees it, unmounted first from `/boot`,
  where the stick mounts it as it starts, and the vault through a handle
  of its own, written back before the step ends, while the install's
  other handle only reads.
- **The screen.** One step at a time, and one thing asked at a time, each
  before the step starts. It's drawn in the theme's roles and marks as the
  build was given them (`theme.nix`), with the console's stand-ins on a
  console, and follows docs/design.md's row for it. Return goes on, Escape
  goes back one ask, Ctrl+C leaves; on the stick that's a root shell.
  Every screen says in words what to do, ending with the key that does it
  ("Type the Nano's PIN, then press Return."), and its foot names every
  key that works there and what it does (Return enter it, Return choose,
  never a bare "go on"), Esc only where it goes back. Keys are drawn by the
  name on their keycap on a console and in a terminal alike, so the guide
  reads the same wherever it runs. A secret with another way in (a
  passphrase, or a YubiKey and its PIN) is one field under both, never
  two screens. Each step done stays listed above the step at hand with
  what showed it done, the latest nearest, the earliest giving way on a
  short screen. A step at work is drawn again once a second, from a
  thread of its own, with what the machine's meter says (`progress.rs`):
  the bytes done of how many, how fast and how long is left, where the
  step feeds it, and otherwise how long it has run, so a wait never looks
  like a hang. The stick's write and read-back count every byte through
  the disk; copying off the stick counts what it writes; a copy a program
  does, as `nix copy` onto the disk, is counted by what that program has
  read, from /proc. A screen saying what's in the way looks at the machine
  again every two seconds, so it goes on by itself once that's done.
- **Running from memory.** A laptop may have one port both the stick and a
  YubiKey fit, so the stick's system is copied into memory before the
  keyboard is checked, and the stick can come out for each key. Its store
  is a dm-verity device, `usr`, over two partitions; both are copied into
  a tmpfs of their own, checked whole with veritysetup against the root
  hash the stick booted with, and handed to the kernel as read-only loop
  devices. `dmsetup resume` swaps the table in one call, suspended around
  the swap, so nothing reads the store while it moves, and every block
  read after is proved by the same hash. A machine without the memory for
  the copies and a gibibyte beside them runs from the stick, and the key
  steps say to keep it in. The vault is named by its partition's GUID once
  opened, and opened again whenever its device node is made afresh, which
  a stick put back always is. What the steps only read (the target, the
  db certificate the firmware is checked against) is held under
  /run/guide when the vault opens, so they go on with the stick out; a
  step that needs the vault itself says to put the stick back, and goes
  on once it's in.
- **The vault.** A FAT32 partition any machine can read, holding files
  sealed with age to a vault key made for it. The key is sealed to each
  YubiKey, and separately to the recovery passphrase, since age won't mix a
  passphrase with other recipients. Files are split into objects of at most
  1 GiB, under FAT32's limit, with numbered names; only the sealed index
  says what they hold, and extracting checks every object's size and each
  file's SHA-256. `manifest.json` goes last, so a vault without one was
  never finished. Every file sits at the top, since fatfs 0.3.6 writes
  directories fsck.fat has to repair.
- **Opening the vault.** The key's YubiKey copy goes through
  age-plugin-fido2-hmac, which age finds only on PATH, so the guide sets
  its own PATH to the plugin's store directory as it starts. Sealing needs
  no key plugged in. On the stick one field takes either the stick's
  passphrase, named as the host had the person save it on their phone, or
  the plugged-in key's PIN; the plugin's own prompts never reach the
  screen, since the PIN is handed to it.
- **Passphrase before PIN.** Wherever one field takes a passphrase or a
  key's PIN (the vault, a rescue bundle, recovering the disk), what's
  typed is tried as the passphrase or recovery key first, which costs the
  key nothing; only then is it the plugged-in key's PIN. Typed the other
  way round, a passphrase with the Nano plugged in would spend one of the
  PIN's eight tries, and a key whose PIN is used up opens nothing until
  it's reset, which wipes what it opened. A key counts as plugged in only
  when `fido2-token -I` says it has hmac-secret and a PIN, set or not, so
  a laptop's virtual key such as linux-id, listed with no key plugged in,
  is never asked for a PIN.

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
| `src/words.rs`        | the secrets a person keeps, as seven words, and how they're kept |
| `src/words.txt`       | EFF's large wordlist, verbatim, its SHA-256 checked by a test |
| `src/observe/`        | one parser per thing the guide reads off a machine        |
| `src/sign/`           | Authenticode: what a signature covers, and the signature sbsign would make |
| `src/stick/`          | writing the stick and reading it back: its tables, and aligned blocks |
| `src/vault/`          | the vault's sealing and index, and where its files are kept |
| `src/host/`           | writing a stick on a host, and what a reinstall stick carries |
| `src/rescue.rs`       | the rescue bundle: a machine's Secure Boot keys sealed, made and opened |
| `src/install/`        | installing a machine from its stick, a step to a file by what it touches |
| `src/installed/`      | finishing the machine on itself: Secure Boot checked, the TPM sealed |
| `src/recover/`        | recovering a machine from a stick: found, opened, mounted |
| `nixos.nix`           | the guide on a machine it installed                       |
| `src/screen/`         | the view, drawn as lines in roles; the terminal; the walk through a journey |
| `payload.nix`         | what a reinstall stick carries for one machine            |
| `stick.nix`           | the stick's own system                                    |
| `theme.nix`           | the theme as the guide and the stick's console draw it    |
| `fixtures/<hardware>/` | what each tool printed on a real machine, as it printed it |
| `tests/journey.rs`    | the journey's promises, with toy steps                    |
| `tests/stick.rs`      | whole sticks written into files, judged by sfdisk, fsck.fat and sbverify |
| `tests/sign.rs`       | signatures compared byte for byte with sbsign's           |
| `tests/system.rs`     | secrets handed to a real program, and kept out of the log |
| `tests/host.rs`       | the one stick written, never another disk              |
| `tests/rescue.rs`     | a bundle read as root, sealed, proven, and refused when its keys aren't its own |
| `../../tests/guide-host.nix` | a stick written on a host through UDisks2, as the person |
| `../../tests/guide-vault.nix` | a vault opened by a YubiKey's PIN, then by the passphrase |
| `tests/recover.rs`    | the machine found by its layout, and shown before anything's asked |
| `../../tests/guide-recover.nix` | a machine recovered from a repair stick to a root shell inside it |
| `../../tests/guide-install.nix` | a machine installed from its stick in a VM, by the guide's keys |

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt. `cargo test` runs in a second, and the package runs the
tests again as it builds, then `guide tools`. `nix fmt` from the repository
root formats the Rust. `guide observe` prints what the guide sees on the
machine it runs on and changes nothing, which is how a parser meets a
machine it hasn't seen. `guide write-stick IMAGE FILE` writes a repair stick
into a file already the size of a stick, and refuses a disk; given
`--sign-with` and sbctl's db key directory, it signs the stick's boot
program for that machine. It carries no rescue bundles, which only
`guide host` gathers and spreads.

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
