# What each check proves, by its name, as a sentence's predicate: checks.nix puts each in its
# check's meta.description, and every-check-says-what-it-proves fails on a check with none or
# one with no check. A check added names what it proves here, in its place by name.
{
  a-claimed-chord-is-refused = ''
    a chord two layers claim, or one claims twice, is refused
  '';
  a-failed-boot-says-what-stopped-it = ''
    a failed boot's shell names up to four failed units and their last lines inside 80 columns, and
    the way out, and nothing when none failed
  '';
  a-faint-theme-is-refused = ''
    a theme with text under WCAG's 4.5:1 doesn't build
  '';
  a-mac-sudo-takes-touch-id = ''
    every Mac's sudo lets Touch ID answer it, in the file macOS updates keep
  '';
  a-prompt-waits-for-a-person = ''
    a D-Bus call from the account's shells, as run0 asks for polkit's prompt in, outlasts systemd's
    25 seconds
  '';
  a-quiet-file-says-nothing = ''
    Quickshell's FileView, told not to print errors, says nothing of a file that opens and refuses
    every read, as one not told still warns
  '';
  a-sender-keeps-to-its-level = ''
    a notice reaches no further than its sender is kept to, however critical, and each level shows,
    counts and stays as it says
  '';
  a-theme-changes-everything = ''
    another theme leaves none of the first's colours behind
  '';
  a-watched-file-comes-back = ''
    the shell's status file hears a status once its folder is made, after the shell starts and again
    after it's removed, and says nothing while it's gone
  '';
  agents-work-runs-without-real-time = ''
    the jobs sorter takes real time from work under jobs.service, jobs-work.slice and
    jobs-timed.slice, and from nothing that holds Joel's own terminal work
  '';
  backlight-passes-clippy = ''
    clippy finds nothing in the backlight daemon or its tests
  '';
  bat-reads-its-theme = ''
    bat colours code in the account's theme from a cache built once, saying nothing, delta finds
    the theme through it too, and a switch builds no cache of its own
  '';
  batteries-leave-time-to-power-off = ''
    UPower powers a laptop off at 5%, before it goes flat
  '';
  bluetooth-keeps-its-power-limits = ''
    the XPS 14's kernel reads Bluetooth's revision 2 SAR table and antenna gain setting from UEFI
  '';
  bluetooth-links-passes-clippy = ''
    clippy finds nothing in bluetooth-links or its tests
  '';
  bluetooth-links-reads-the-replies = ''
    bluetooth-links reads the kernel's lists of adapters and of the connections that are up, with
    who leads each, who started it and how it's secured, and each adapter's address and byte
    counts, writes each HCI command as the controller takes it and picks its answer out of the
    events, reads the signal, link quality, power, channel map and the adapter's version and maker
    from reply bytes, leaves a failed one null, keeps each link's last twenty reads and each
    adapter's traffic, asks the version once, writes nothing new while nothing is connected, and
    replaces its file whole, readable by everyone
  '';
  bluetooth-pairs-before-it-connects = ''
    the launcher trusts a Bluetooth device before pairing it, follows it past its link coming up
    until it's paired, untrusts one that fails, and connects one paired before; lists a device's
    parts after its verbs, named first: its battery warning low, its signal and link quality as
    sparks of the reads bluetooth-links would write, its channels, connection, an input device's
    facts held by how it's carried, whether it reconnects by itself and wakes the machine as
    settings whose choices set them, and its address to copy; lists the adapter's radio,
    visibility and pairing as settings, its traffic as a spark and what it is; and answers how the
    links are doing
  '';
  boot-lock-is-whole = ''
    the splash's lock, at each scale, holds all the ink its glyph has drawn with room to spare, and
    is a line of text tall
  '';
  booted-generation-stays-installed = ''
    a switch never removes the generation the machine booted
  '';
  btop-draws-in-the-theme = ''
    btop draws in the theme's colours, not its own
  '';
  btrfs-reclaims-its-chunks = ''
    tmpfiles turns Btrfs's reclaim on for data and metadata
  '';
  call-audio-runs-in-real-time = ''
    an app's audio thread can take SCHED_RR 8, below PipeWire's, for at most 200 ms at a time
  '';
  cargo-links-with-mold = ''
    cargo links an account's Rust builds with mold
  '';
  changed-checks-names-what-moved = ''
    changed-checks names a check added, changed, removed or broken by its derivation, fails on a
    broken one, builds those added and changed when asked, leaving out the VM tests with --no-vm,
    and evaluates a commit's checks once
  '';
  changed-checks-passes-clippy = ''
    clippy finds nothing in changed-checks or its tests
  '';
  chromium-leaves-the-panel-to-niri = ''
    the XPS 14's Chrome, Slack and 1Password leave its panel's colours to niri, which maps sRGB onto them
  '';
  claude-shows-its-status-line = ''
    Claude Code is told of its status line on Linux and macOS, in settings it can still write
  '';
  claude-status-line-says-how-it-stands = ''
    Claude Code's status line shows the folder, the model and the context's size, cached tokens
    counted, in the theme's colours, and the command a workstation gives Claude Code draws the
    folder in the theme's strong colour
  '';
  claude-statusline-passes-clippy = ''
    clippy finds nothing in Claude Code's status line or its tests
  '';
  claude-wears-the-theme = ''
    Claude Code's theme has the account's text colour and only hex colours, and the mix its tints
    come from is exact
  '';
  clipboard-history-passes-clippy = ''
    clippy finds nothing in the clipboard history or its tests
  '';
  console-font-passes-clippy = ''
    clippy finds nothing in the console's font tool or its tests
  '';
  darwin-role-twice-is-once = ''
    importing a nix-darwin role twice leaves the machine unchanged
  '';
  dell-events-are-known = ''
    the XPS 14's dell-wmi ignores the EC's hourly type 0x0012 event rather than log an unknown key
  '';
  desk-facts-read-the-desk = ''
    desk-facts reads a monitor's EDID and USB-C cable, a USB sound card's modes, a built-in
    card's jacks and chips, a camera's formats, a loopback camera's mode and sensor, a
    Bluetooth input device's link and inputs, and a wired keyboard's or mouse's polling rate and
    inputs as the devices' own report them, and writes the launcher's JSON in order
  '';
  device-check-passes-clippy = ''
    clippy finds nothing in device-check, device-check-disk or their tests
  '';
  devices-that-stop-working-are-named = ''
    `device-check` names a failed or missing device, or a disk's fault, once, and again when it's
    fixed
  '';
  dictation-hears-the-clip = ''
    Whisper on the CPU types a public clip back word for word
  '';
  dictation-passes-clippy = ''
    clippy finds nothing in dictation or its tests
  '';
  dictation-says-what-its-doing = ''
    dictation's surface says listening with the time, counts down from 25 s, says typing once it
    lasts, still writing, and why it heard nothing; its wave keeps the loudest peak of each step
  '';
  dictation-stops-by-its-chord = ''
    a chord runs `dictation hold`, so dictation can be held to talk
  '';
  dictation-switches-to-the-new-one = ''
    a switch restarts dictation after reloading its units
  '';
  dictation-works-it-out = ''
    dictation's spectrogram, transform and vocabulary match their references, words turn into
    Whisper's tokens as GPT-2's merges join them; a held chord's release stops only what it
    started, and a tap's none
  '';
  dictation-writes-the-words-it-knows = ''
    Whisper writes a name it misheard as the words file spells it, given the file
  '';
  disabling-a-module-removes-it = ''
    `disabledModules` removes a module a role brought in
  '';
  disk-guard-accepts-its-configuration = ''
    the disk guard passes its scenarios and accepts the configs a workstation and its account get
  '';
  disk-guard-asks-github-what-merged = ''
    the disk guard counts a GitHub checkout finished only once its own head was merged
  '';
  disk-guard-compiles-for-macos = ''
    the disk guard and its scenarios compile for macOS, clean
  '';
  disk-guard-finds-a-machines-programs-first = ''
    a machine's programs for the disk guard come first on its PATH under systemd and launchd, with
    core's git after
  '';
  disk-guard-passes-clippy = ''
    clippy finds nothing in the disk guard or its scenarios
  '';
  disk-guard-stays-in-the-background = ''
    the disk guard yields to the account, and a command overrunning its memory never stops it
  '';
  disks-are-scrubbed = ''
    the disk's Btrfs is scrubbed monthly, from its root
  '';
  disks-go-by-their-name = ''
    a disk's partitions and open volume carry its name
  '';
  disks-say-when-they-fail = ''
    a workstation runs smartd on its disks and hands their faults to device-check, not to every
    terminal
  '';
  do-not-disturb-holds-while-sharing = ''
    do not disturb turns on while the screen is shared, not recorded, and off after; ended mid-share
    it stays off until the next, and one kept by hand outlasts it
  '';
  each-machine-signs-with-its-hardware = ''
    a Linux workstation signs through its TPM and a Mac through its Secure Enclave, both with one
    program and Joel's key by default, and signing alone, with no hardware, signs nothing
  '';
  eero-passes-clippy = ''
    clippy finds nothing in eero or its tests
  '';
  eero-reads-what-eero-says = ''
    eero reads a network's line, speed test, conferencing setting and connected devices from what
    eero's cloud answers, a missing field as unknown, and tells an aged session from other refusals
  '';
  ergodox-on-every-workstation = ''
    every workstation can flash the ErgoDox, a Mac as much as Linux
  '';
  every-check-says-what-it-proves = ''
    every check says here what it proves, and every description here names a check
  '';
  docs-name-what-exists = ''
    every doc's links and paths name what's there, but the exemptions named
  '';
  every-folder-says-what-it-holds = ''
    every folder says what goes in it, but those still waiting for their note
  '';
  example-darwin = ''
    a Mac workstation evaluates
  '';
  example-laptop = ''
    a laptop on the XPS 14, its disk and Secure Boot, evaluates
  '';
  example-nixos = ''
    a NixOS workstation with a screen evaluates
  '';
  firmware-leaves-the-keyboard-light = ''
    the XPS 14 leaves the keyboard light to its key, still holding its other firmware settings
  '';
  firmware-settings-passes-clippy = ''
    clippy finds nothing in firmware-settings or its tests
  '';
  formatting = ''
    everything is formatted and passes the linters
  '';
  gimp-accepts-its-configuration = ''
    GIMP reads its preferences, the theme's palette and colours
  '';
  git-engine-agrees-with-gitstatusd = ''
    git-engine answers as gitstatusd does across a prompt's repository states, counting all and
    stopping at one, under git's monitor too, and as git does where gitstatusd can't read
  '';
  git-engine-passes-clippy = ''
    clippy finds nothing in git-engine or its tests
  '';
  git-settings-stay-writable = ''
    programs write git's settings and the declared ones still win
  '';
  greetd-waits-for-the-gpu = ''
    the XPS 14's greetd starts once the GPU's device unit, tagged by udev, appears, waiting at most
    15 s, rather than for the boot to go idle as elsewhere
  '';
  guide-compiles-for-macos = ''
    clippy finds nothing in the guide as macOS compiles it
  '';
  guide-passes-clippy = ''
    clippy finds nothing in the guide or its tests
  '';
  guide-proves-its-steps = ''
    the guide counts a step done only once observing it again says so, reads what real machines'
    tools printed, writes sticks that sfdisk, fsck.fat and their own key read back whole, signs EFI
    programs byte for byte as sbsign does, and finds every program it runs
  '';
  icon-marks-past-the-basic-plane = ''
    an icon mark past U+FFFF reaches the shell whole
  '';
  inkscape-accepts-its-configuration = ''
    Inkscape starts a drawing on the theme's desk and reads its stylesheet
  '';
  ipu7-outlives-its-open-files = ''
    the XPS 14's capture side is freed when its last node closes, not when the IPU unbinds
  '';
  ir-camera-passes-clippy = ''
    clippy finds nothing in ir-camera or its tests
  '';
  jobs-sorter-passes-clippy = ''
    clippy finds nothing in the jobs sorter or its tests
  '';
  jobs-sorter-sorts-by-behaviour = ''
    the sorter moves a group busy two seconds running, never the shell's or pi's, and sends a scope
    started from jobs back there unless it sets limits
  '';
  joels-work-goes-before-agents = ''
    the sorter moves a terminal's work inside jobs, above pi's work and below timed work
  '';
  kernel-settings-hold = ''
    the kernel's patches apply and every setting holds
  '';
  kernel-settings-hold-dell-xps-14-da14260 = ''
    the XPS 14's own kernel applies its patches and holds every setting
  '';
  kernel-settings-hold-microsoft-surface-laptop-4-1958 = ''
    the Surface Laptop 4's own kernel applies its patches and holds every setting
  '';
  kernel-settings-hold-with-a-theme = ''
    the kernel holds every setting with an account's theme, its panic colours among them
  '';
  kernel-trouble-is-named = ''
    `device-check` names a kernel that oopsed, locked up, saw a hardware fault or held a task stuck,
    once, and not its harmless taints
  '';
  kitty-accepts-its-configuration = ''
    kitty loads its files without a complaint
  '';
  kitty-draws-the-ligatures = ''
    kitty draws !=, >= and -> as the mono face's ligatures with the account's fontconfig features,
    and |> and 1..=2 plain
  '';
  lag-watch-passes-clippy = ''
    clippy finds nothing in lag-watch or its tests
  '';
  lag-watch-tells-who-waited = ''
    lag-watch counts the desktop or apps waiting as trouble, the desktop's memory stalls only from a
    120 Hz frame's worth of the window, and its waits on the disk or a page being moved from the
    same, niri's turns for a CPU only past 3% of a frame, and work waiting on work as none, names
    the app's unit that stalled most, tells of the desktop's or an app's program the kernel ended
    for memory, names in a word the wait felt most, whose it was and since when trouble has lasted,
    finds the threads it watches by name, of niri only the one its socket names, and sums each
    generation by its load
  '';
  lid-docked-passes-clippy = ''
    clippy finds nothing in lid-docked or its tests
  '';
  load-proof-counts-frames-missed = ''
    load-proof counts a refresh with the probe's frame ready and none shown as a frame niri missed,
    one that passed before the probe committed as the probe's own, none when a frame lands at every
    refresh, and a frame niri dropped for a later one apart
  '';
  load-proof-passes-clippy = ''
    clippy finds nothing in load-proof or its tests
  '';
  machines-yield-to-what-the-user-works-in = ''
    system virtual machines are weighed as a build is, and the account's containers and session
    machines are idle
  '';
  memory-modules-passes-clippy = ''
    clippy finds nothing in memory-modules or its tests
  '';
  memory-modules-reads-the-firmware = ''
    memory-modules reads each memory device's locator, size, type, speed, maker and part from the
    bytes the firmware lists, leaves out an empty slot, and reads LPDDR5 past 6400 MT/s as LPDDR5X
  '';
  mona-sans-vf-is-the-whole-face = ''
    the graphical role's fontconfig, asked for Mona Sans VF by name as GitHub asks, gives the variable
    file with every axis and lists no other file under that name
  '';
  mpv-accepts-its-configuration = ''
    mpv plays a moment of video with the account's settings and its controller's without a
    complaint, and video files open in it
  '';
  neovim-binds-every-key = ''
    Neovim binds every key keys.nix names and only those, each action provided, in its scope's
    buffer or plugin window, completion and the command line included
  '';
  neovim-draws-its-status-line = ''
    Neovim's status line names the file from its repository's root, the branch and its unsaved mark,
    a change, an error, the mode, a version and a terminal, and fits 35 columns
  '';
  neovim-finds-a-documents-pictures = ''
    Neovim finds a markdown file's picture by its path relative to the file, for the terminal to
    draw, with ImageMagick there for pictures other than PNG
  '';
  neovim-finds-in-every-group = ''
    every key group in Neovim has its own Space, which lists in one column, titled, the group's keys
    that work here and runs the one chosen, or says none does
  '';
  neovim-floats-by-shade = ''
    Neovim's floats, finder and suggestions have a blank border, not a line, on their shade with
    nothing showing through; a notice starts with its mark
  '';
  neovim-keeps-its-numbers-in-one-column = ''
    Neovim's number column gives the cursor line its own number, marked, as wide as the relative
    counts beside it, and is blank with numbers off
  '';
  neovim-knows-the-names-it-writes = ''
    Neovim spell-checks markdown without flagging niri, Quickshell, YubiKey, TPM or monospace, and
    still flags a misspelt word
  '';
  neovim-starts-in-the-theme = ''
    Neovim starts in bamboo, transparent, without a complaint
  '';
  neovim-starts-with-only-what-it-draws = ''
    Neovim opens a file with only what draws its first screen, every key's command standing in,
    loads the rest when used or once drawn, and lua_ls reads only its runtime
  '';
  network-revive-passes-clippy = ''
    clippy finds nothing in network-revive or its tests
  '';
  niri-accepts-its-configuration = ''
    niri loads the files an account gets
  '';
  niri-cursor-has-its-shapes = ''
    the cursor theme niri names has the shapes programs ask for
  '';
  niri-never-shows-its-own-screens = ''
    no niri key opens its screenshot screen, its key list or a quit that asks, and the launcher's
    Log Out quits without asking
  '';
  niri-says-what-the-gpu-gave-it = ''
    the session's niri lets smithay say which GPU priority niri got, keeping the rest of niri's own
    log filter
  '';
  niri-starts-able-to-go-first = ''
    the session's niri starts through its wrapper, which holds CAP_SYS_NICE
  '';
  nix-builds-yield-to-the-user = ''
    Nix's builds run at a lower weight than the user's programs
  '';
  nix-collects-before-the-disk-fills = ''
    Nix collects garbage during a build that fills the disk
  '';
  nixos-role-twice-is-once = ''
    importing a NixOS role twice leaves the machine unchanged
  '';
  nothing-names-its-own-look = ''
    no module but the theme's names a colour or a face
  '';
  nothing-spells-a-mark = ''
    no module but the theme's spells a mark's code point instead of naming it
  '';
  one-niri = ''
    the screens go off through the machine's patched niri, not a second one
  '';
  one-ruleset-for-every-program = ''
    the firewall, libvirt and Podman share nftables, and a reload flushes none of them
  '';
  one-ssh-agent = ''
    a workstation with a screen answers SSH with the TPM only
  '';
  onepassword-works-with-chrome = ''
    1Password has its Chrome helper and unlocks with polkit
  '';
  only-external-monitors-reach-the-seat = ''
    ddcutil's rules give the seat a DisplayPort connector's AUX bus and an MST branch's, and keep
    the panel's eDP, HDMI's, the GPU's own and the touchpad's buses root's, ahead of seat-late
  '';
  open-with-passes-clippy = ''
    clippy finds nothing in the portal's chooser or its tests
  '';
  palette-graphs-draw-in-braille = ''
    a row's graph is ten braille characters over a dim track, a bar from the left, a spark from the
    right and a strip a reading a member, warning at 95%; graphs line up in one column, and a
    figure wears no glyph
  '';
  palette-rows-keep-their-words-apart = ''
    a palette's row and field keep their words apart and inside them, however long, ending what
    doesn't fit in an ellipsis
  '';
  pebble-lights-follow-the-sound = ''
    pebble-lights finds the speakers' control and sound card, keeps their lighting on while sound
    plays and a while after unless the screen is locked, and lets the speakers' own button win
  '';
  pi-ask-answers-on-the-machines-plan = ''
    pi-ask starts pi for the launcher's Ask row speaking RPC with nothing loaded but the machine's
    extensions, on pi's own plan and quick model by default, or through pi-claude on the Claude plan
  '';
  pi-claude-keeps-to-the-plan = ''
    pi-claude starts pi with Claude Code's provider, on Sonnet unless a model is asked for, off the
    mesh, signed in to the plan's own folder and with nothing that would bill an API instead
  '';
  pi-keeps-its-own-settings = ''
    pi's settings take core's at a switch and keep pi's own keys; a file holding them isn't
    rewritten, a missing one is made, and one not JSON is refused untouched
  '';
  pi-tells-where-as-the-prompt-does = ''
    pi's status line finds the folder by the prompt's roots, asks the prompt's git-engine, and marks
    git with the theme's glyphs
  '';
  pi-wears-the-theme = ''
    pi's text, strings and deepest thinking are the account's text, string and alert colours, and
    every colour it names is hex
  '';
  platform-profiles-reach-the-legacy-file = ''
    every profile Dell's driver offers, the XPS 14's SoC slider takes too
  '';
  plymouth-waits-passes-clippy = ''
    clippy finds nothing in plymouth-waits or its tests
  '';
  pnpm-keeps-one-store = ''
    pnpm 10 and 12 find an account's one store from what its shells and user manager are given
  '';
  qemu-starts-after-its-sockets = ''
    the qemu VM tests run starts once the sockets it connects to are there, and fails naming one
    that never comes, ending what its script started
  '';
  qt-programs-wear-the-theme = ''
    Qt programs in the session take GTK's platform theme, and Quickshell is started without it
  '';
  quickshell-opens-programs-through-niri = ''
    programs the shell opens survive the shell restarting
  '';
  quickshell-passes-qmllint = ''
    qmllint finds nothing in the account's shell or the greeter, leaving out only types Quickshell's
    files don't export and Qt enum parameters
  '';
  quickshell-restarts-with-its-files = ''
    a switch restarts the shell when a file it reads changes
  '';
  quickshell-takes-its-shape-from-the-theme = ''
    every corner and border in the shell is the theme's
  '';
  record-window-passes-clippy = ''
    clippy finds nothing in record-window or its tests
  '';
  resolved-is-set-per-network = ''
    a laptop leaves resolved's global DNS settings as they ship
  '';
  run-subtests-keeps-what-is-named = ''
    run-subtests keeps a script's boot and helpers and only the subtests named, and runs a pytest
    test whole with pytest picking the tests named, refusing a word pytest would read as more
  '';
  rust-programs-pass-clippy = ''
    clippy finds nothing in the single-file programs or their tests
  '';
  se-keys-builds-for-any-name = ''
    the Mac's key maker builds, shellcheck and all, for a name with an apostrophe
  '';
  session-machines-have-a-bridge = ''
    libvirt's bridge helper lets session machines onto virbr0, and its default network starts at
    boot
  '';
  session-memory-is-protected = ''
    every slice above session.slice carries a hard memory floor
  '';
  shaper-finds-the-rate = ''
    the shaper reads a queue from delayed pings, not one late ping or a silent host; cuts under what
    the line carried, holds while it drains, and climbs while it's quiet, past what it held only
    once busy five seconds, so bursts fit under the line; forgets a cut ten quiet minutes old, and
    keeps one the line then proves; cuts no download arriving faster than its rate, nor beside one
    an upload carrying under half the line's best, nor again for a queue back within two seconds;
    climbs back from a cut no queue returns for, so late pings alone don't wear an upload down, yet
    retries no cut near one confirmed; never cuts Wi-Fi more than 5% under the best it carried,
    while following a cable down, unless a busy direction's queue stands six seconds at that floor,
    or 40% while real-time traffic flows, which a call's packets turn on for 30 s; remembers the
    line's best, not its latest cut; reads sets, routes and queues from what the kernel prints
  '';
  shaper-passes-clippy = ''
    clippy finds nothing in the shaper or its tests
  '';
  shell-preview-follows-the-working-copy = ''
    shell-preview links each built file the working copy holds back to it, copies what the build
    made, and ends the nested desktop it started however it's ended, asking it before killing it
  '';
  shell-preview-passes-clippy = ''
    clippy finds nothing in shell-preview or its tests
  '';
  shutdown-status-passes-clippy = ''
    clippy finds nothing in shutdown-status or its tests
  '';
  shutdown-status-reads-the-last-stage = ''
    shutdown-status reads only the last stage's steps, says forced once when it begins unmarked, and
    leftovers after three quiet seconds
  '';
  smartd-stops-cleanly-without-disks = ''
    the workstation's own smartd command, run with no disks, stops cleanly rather than failing
  '';
  sound-hears-only-what-can-be-heard = ''
    the launcher's Sound leaves out a jack with nothing plugged in and a display's output with no
    display, and never listens to a headset's microphone while it plays music
  '';
  speaker-amps-have-every-dai-named = ''
    the XPS 14's SoundWire machine driver names only DAIs its amplifiers register
  '';
  spend-watch-accepts-its-configuration = ''
    spend-watch passes its tests and accepts the config an account gets, a feed and a cap among it
  '';
  spend-watch-passes-clippy = ''
    clippy finds nothing in spend-watch or its tests
  '';
  spend-watch-stays-in-the-background = ''
    the AI spend watch yields to the account, comes back when it fails, and leaves its status behind
    when it stops
  '';
  spotify-draws-in-the-theme = ''
    Spotify's client is built in the theme's colours and faces
  '';
  spotify-player-accepts-its-configuration = ''
    spotify-player starts with its settings
  '';
  stands-alone = ''
    nothing here names a machine, its repository or employer
  '';
  swayimg-accepts-its-configuration = ''
    swayimg draws a picture with the account's settings and the machine's fonts without a complaint,
    and pictures open in it
  '';
  swayimg-moves-by-its-keys = ''
    in swayimg, pressed as a keyboard presses them, → and ← go to the next and previous picture and
    End and Home to the last and first
  '';
  the-askpass-outlives-its-shell = ''
    the askpass asks a restarted shell again, never asks twice what's answered, and refuses about 20
    s after the shell goes unless it comes back
  '';
  the-askpass-says-who-asks-for-what = ''
    the askpass names the program and the key, passkey or host behind each prompt, and whether it
    wants a yes, a secret, words or a touch
  '';
  the-camera-gives-limited-range = ''
    the XPS 14's camera relay, run on a full-range ramp, squeezes it exactly into limited range and
    neither of its converters changes it after
  '';
  the-camera-holds-no-memory-it-never-uses = ''
    neither half of the XPS 14's camera asks at probe for a firmware log only Intel's offline logger
    writes, and the capture side frees its firmware's memory when it unbinds
  '';
  the-camera-takes-its-modules-tuning = ''
    the XPS 14's camera reads Dell's tuning for the module a machine names, and Intel's otherwise
  '';
  the-click-strength-finds-the-intensity = ''
    the click strength's setter finds a haptic pad's intensity, the XPS 14's among them, and writes
    only it, only when it differs
  '';
  the-click-strength-holds-after-sleep = ''
    a machine naming a click strength sets it as each hidraw device appears and after every sleep,
    and one naming none runs nothing
  '';
  the-colour-camera-says-it-is-upside-down = ''
    the XPS 14's camera bridge reports its colour sensor rotated 180 degrees
  '';
  the-console-greets-the-account = ''
    a console greets the account by its description's first word, or its name, with the keys back to
    the desktop, and keeps NixOS's banner with no account
  '';
  the-console-font-refuses-a-stand-in = ''
    the console font's build draws the face asked for at Medium, among its other names too, and
    refuses another face or one without a Medium, saying what fontconfig offered
  '';
  the-console-wears-the-face = ''
    the initrd's console font is the account's mono face at Medium, PSF 2 at 16 by 32 with at most
    256 glyphs and a blank slot 32; none without an account
  '';
  the-console-wears-the-theme = ''
    the console takes the account's terminal colours in its own order, whatever the theme, and a
    machine with no account keeps the kernel's
  '';
  the-desktop-draws-letters-unhinted = ''
    the graphical account's desktop settings tell GTK and Chrome to draw letters unhinted, as macOS
    does
  '';
  the-disk-piece-reads-the-guards = ''
    the disk piece says the worst disk as the guards wrote it, and a guard gone quiet
  '';
  the-ergodox-command-reaches-the-board = ''
    `ergodox` writes each command whole to the board's channel alone
  '';
  the-ergodox-layout-builds = ''
    the layout fits the board and answers with its own serial
  '';
  the-ergodox-watch-tells-the-layer = ''
    the layer watcher finds the board and says only what it's told
  '';
  the-firmware-takes-what-is-declared = ''
    `firmware-settings` names what differs, writes it with the password, then wipes it, even when
    stopped partway, and refuses a value it can't read
  '';
  the-glossary-names-each-thing-once = ''
    no word in docs/glossary.md is the name, another word or a replaced name of two things, and
    each clash is printed, and the check refuses a stand-in glossary breaking each of its rules
  '';
  the-gpu-security-link-loads-with-the-gpu = ''
    the XPS 14's initrd loads mei_me and mei_gsc_proxy with xe, inside the 20 s xe waits for its
    CSME link
  '';
  the-guide-keeps-one-accounts-spare-on-the-yubikeys = ''
    the installed guide keeps the spare SSH key of the one account `jitsusama.yubikeySpare` names
    on each YubiKey, and a machine where two accounts want one is refused
  '';
  the-guide-makes-the-keys-the-modules-name = ''
    the installed guide makes every TPM key ssh-tpm-agent's and tpm-signing's modules name, for
    the account naming it, with the PIN each asks for
  '';
  the-keyboard-light-is-heard = ''
    the shell finds the keyboard's light among the kernel's and hears each change its firmware
    makes, once, with its level, off as dark
  '';
  the-launcher-answers-the-rain = ''
    the weather's parts and the rain by the hour come from one forecast, and "rain", "umbrella"
    or "when will it rain" answer when it next rains, here or in a place; here is where the laptop
    is, by its position, ahead of the place set and then the time zone, and the footer says which
  '';
  the-launcher-answers-what-is-asked = ''
    sums, addresses, measures, zones, colours, the machine's figures and its parts' heat answer
  '';
  the-launcher-finds-emoji = ''
    emoji are found by name and meaning, best first
  '';
  the-launcher-finds-the-desk-light = ''
    the launcher says how the desk light is lit and sets its brightness, warmth and power through
    litra, and the bar hears changes but not a camera putting it out
  '';
  the-launcher-finds-the-keyboards-keys = ''
    a key is found by what it types, and shown where it is, and the board's Layer and Lights hold
    choices that run its own command
  '';
  the-launcher-finds-the-people = ''
    the launcher's Tuple place leads with the room or a call's controls, lists and finds contacts as
    they stand, calls, joins and favours them, and never starts a stopped Tuple
  '';
  the-launcher-finds-the-quests = ''
    the launcher lists open quests driving first, finds finished ones and ids, opens pi on one in
    its tree or goes to its session, and sees one made while it was shut
  '';
  the-launcher-finds-the-runs = ''
    the launcher lists the day's fleets and councils, running first, with progress or cost and time,
    goes to their session, opens an answer whole, and hides amounts while sharing
  '';
  the-launcher-finds-what-is-meant = ''
    the right row comes first for each query, within budget
  '';
  the-launcher-groups-every-shortcut = ''
    every shortcut of every layer lands in one place in Shortcuts, a desktop chord always in a group,
    a new chord finds a group of its own accord, every group is named in the vocabulary, and
    Shortcuts steps from the layers to their groups to the chords, typing finding any of them flat
  '';
  the-launcher-holds-a-programs-controls = ''
    a program's step holds its windows, its own actions, Quit asked once, then Media, Sound and
    Notifications, in order and only where they apply, each reaching only that program
  '';
  the-launcher-holds-still-as-it-refreshes = ''
    a list the launcher shows keeps its order and its selection as what's in it changes
  '';
  the-launcher-keeps-its-trials = ''
    each opening of the launcher is kept as its keys by kind, its pick and how long it took, and a
    miss only where letters were typed and nothing picked
  '';
  the-launcher-lands-where-it-did = ''
    a thing is one row, the same letters keep their row, one slip teaches nothing
  '';
  the-launcher-means-one-thing-a-word = ''
    no name, word or question in the launcher's vocabulary finds two things at once unless it's
    listed as meant to, and each clash, unknown name, missing glyph and thing with no place is
    printed, and the check refuses a stand-in vocabulary breaking each rule of what Joel reads
  '';
  the-launcher-names-the-parts-it-builds = ''
    every part the calculator, the time, today, the waits and a hardware address show is named as
    the vocabulary names it
  '';
  the-litra-follows-the-cameras = ''
    `litra` finds the lights by their ids, speaks their reports, and counts a camera streaming only
    by a whole file naming a program
  '';
  the-mac-guards-its-disk = ''
    a Mac's guards run under launchd, with a Mac's paths and nothing to confine a run
  '';
  the-macs-faces-are-the-themes = ''
    the graphical role's fontconfig, asked for a Mac's system or monospaced font by a name pages give
    it, gives the theme's sans or monospaced face, carrying the name asked for
  '';
  the-machine-lists-processes-for-its-parts = ''
    the processors' and the memory's headlines make no parts and spawn nothing, and their parts,
    once looked at, list the processes once
  '';
  the-margins-glyphs-stand-level = ''
    every glyph in the margin stands one height on the gap's middle line, read from the right by
    kind, in the colour that says how much it asks; what was on air lingers, what a notice will say
    waits for its card, and a wait shows once the same one lasts; each stands in a square cell the
    gap's size with the same room all round, and What's on points at the row the keys are on
  '';
  the-margins-marks-are-drawn-level = ''
    every mark the margin can show is the theme's drawing of it, standing the grid's full height and
    centred across it as Qt draws it
  '';
  the-music-follows-the-player = ''
    the music is the player that started last, never playerctld's echo
  '';
  the-notices-keep-their-order = ''
    the notices' stack puts a critical one in front and the rest in the order they came, one unheard
    at the back, and turns and brings one forward as the keys and a click ask
  '';
  the-notices-land-on-niri = ''
    the account's notices, on its real niri nested in a headless sway in the sandbox, draw a notice
    sent as a card on niri's notices layer, and what Super+Shift+N runs puts a critical one away
    unread in the history
  '';
  the-notices-wait-while-joel-is-away = ''
    a notice that comes while nothing has been typed or moved stays on niri past the time its sender
    gave it, never counting down
  '';
  the-npu-finds-its-compiler = ''
    the XPS 14's graphics drivers hold the NPU's driver and Intel's compiler beside it
  '';
  the-overview-takes-its-pieces = ''
    a machine moves, cuts, retunes and adds the overview's pieces
  '';
  the-panel-dims-after-a-bad-read = ''
    the XPS 14's kernel keeps to VESA's brightness controls when its boot-time read of the panel's
    eDP revision comes back wrong
  '';
  the-panel-knows-its-luminance = ''
    the XPS 14's kernel reads the panel's luminance range from DisplayID's Display Parameters block
  '';
  the-panel-offers-variable-refresh = ''
    the XPS 14's kernel reads the panel's refresh range from DisplayID's Adaptive Sync block
  '';
  the-panel-slows-on-a-low-battery = ''
    the XPS 14's panel drops to 60 Hz below 30% on battery, holds there, and comes back to its
    fastest rate on the charger
  '';
  the-panic-screen-wears-the-theme = ''
    the kernel's panic screen draws the account's text colour on its background, whatever the theme,
    and keeps the kernel's with no account
  '';
  the-picker-picks-what-the-keys-say = ''
    the region picker starts on the middle quarter, moves and sizes its box by arrows within the
    screen, and names a 16 by 10 grid's cells by two letters
  '';
  the-pinentry-asks-the-shell = ''
    linux-id's pinentry asks the askpass in the program's words
  '';
  the-power-profile-answers-before-the-network = ''
    the power profile daemon's unit, the XPS 14's patched one too, waits for neither the network nor
    the boot's end, and keeps upstream's settings
  '';
  the-profile-saves-power-on-a-low-battery = ''
    the power profile follows the charger, goes to power saver below 20% on battery, and holds a
    profile picked by hand
  '';
  the-prompt-gives-way = ''
    from 120 columns to 16 the prompt keeps the folder and glyph, leaves room to type and marks what
    it hides, with refs in files or reftable
  '';
  the-prompt-starts-nothing = ''
    the prompt starts only git-engine and its mkfifo, then forks nothing across prompts in and out
    of a repository, reftable included
  '';
  the-shell-follows-dictation = ''
    the shell follows each dictation state, watches again a service its socket restarts, and tries a
    socket nothing listens on once, never looping
  '';
  the-shell-follows-tuple-calls = ''
    the shell reads from Tuple's log lines a call's start, end and length, who's in it, sharing or
    ringing, Joel's mute, and a call's link
  '';
  the-shell-hears-tuples-call = ''
    the shell's Tuple reads a call from tuple-people's stream, who's in it, since when, mutes,
    screens and cameras, tells comings and goings, and wakes on Tuple's log
  '';
  the-shell-reads-the-spend-watch = ''
    the AI spend piece and collection draw what the watch wrote, put what wants a look first, step
    into each figure's parts, hide amounts while sharing, and answer reports by their words
  '';
  the-shell-speaks-its-javascript = ''
    the shell calls no method Quickshell's JavaScript lacks
  '';
  the-shell-starts-nothing = ''
    the account's zsh starts only git-engine, its fifo and direnv, and each prompt after runs direnv
    alone
  '';
  the-shutdown-mark-cannot-stop-a-reboot = ''
    the mark a healthy stop leaves can't fail the reboot, and what reads it outlives the last stage
  '';
  the-sign-in-consoles-wear-the-theme = ''
    the sign-in consoles are kmscon in the account's mono face, its padding and terminal colours,
    doubled on the XPS 14, and Ctrl+Shift+plus zooms
  '';
  the-signer-picks-who-signs = ''
    git's signing program signs with Joel's key at a terminal and the agent's over pipes or with
    SIGN_AS, and takes back a signature that doesn't verify
  '';
  the-splash-keeps-systemd-quiet = ''
    with the splash, systemd writes only failures to the console
  '';
  the-splash-reads-the-generation-from-the-entry = ''
    the splash is told the generation only from an entry whose name carries one
  '';
  the-surface-keyboard-types-at-the-disk-prompt = ''
    the Surface Laptop 4's initrd has every driver on its keyboard's path, and keyd lays that
    keyboard out there
  '';
  the-window-buffers-stay-under-load = ''
    the XPS 14's udev sets xe's `shrinker_non_purgeable_seeks` to 64, so reclaim under a build
    leaves window buffers alone
  '';
  thermald-starts-after-its-drivers = ''
    the XPS 14's thermal drivers load before thermald starts, and a thermald that stops is restarted
  '';
  timed-work-goes-ahead-of-other-work = ''
    timed work sits inside jobs above its holder, held by a unit that comes back, and pi finds it
  '';
  tuple-completes-in-zsh = ''
    Tab in zsh offers tuple's commands and subcommands, through Tuple's own completion and the real
    tuple
  '';
  tuple-keeps-its-logs = ''
    Tuple's service keeps the daemon's last three logs, newest first, before it starts, and an empty
    log pushes none out
  '';
  tuple-opens-its-links = ''
    xdg-open and gio send tuple:// links to Tuple's entry, which joins a call's link, starts Tuple
    bare, and refuses any other link with a notice
  '';
  tuple-shares-through-the-portal = ''
    Tuple reads the portal setting where the switch writes it
  '';
  tuple-takes-no-real-time-priority = ''
    Tuple's daemon may take no real-time priority, and goes first by a nice below 0 and a CPU weight
    above 100
  '';
  usb-camera-holders-follow-the-stream = ''
    a USB camera's watch names its holders only while a streaming interface has a nonzero alternate
    setting, names its file for the camera, made a plain file name, and removes it when the node
    goes or the watch is stopped; a holder's name is one printable line, and only a unit in its
    cgroup is left out
  '';
  v4l2loopback-holders-passes-clippy = ''
    clippy finds nothing in the cameras' holders watcher or its tests
  '';
  vm-admins-switch-unasked = ''
    an account in wheel switches to another system and back with no password; one outside wheel, and
    what isn't a NixOS system in the store, is refused
  '';
  vm-admins-tune-the-work-slices = ''
    an account in wheel sets build.slice's and machine.slice's weight and cores for the boot with no
    password; one outside wheel can't, and no other unit is opened
  '';
  vm-apps-come-first = ''
    with no disk named, the account's app and session slices, and the system's user.slice, hold a 1
    ms latency on the drive behind /home, through the kernel (jobs, background, machine and system
    none); with one named, io.cost holds its model and qos, no slice holds a target, one left from
    before is cleared as io.cost turns on, and stopping its `disk-order-<disk>` unit turns io.cost
    off; every slice's io.weight follows its CPU weight;
    app.slice holds a hard 13% of memory, session.slice 1.5 GiB and every slice above them 16%, and
    jobs is slowed past half; timed work sits at 10000 inside idle jobs, and work in jobs can move
    itself there; a scope work in jobs starts goes back to the scope of the work that started it,
    and one with a memory ceiling stays; each command written into jobs.service gets a scope of its
    own in jobs-work.slice, at weight 100, with its own swap, and the holder stays; one naming a
    quest pi's sessions weigh goes in that quest's slice under jobs-work.slice at its weight for
    CPU and disk, which follows the file as it changes, and one naming none or an unlisted quest
    stays; two quests' work on one CPU shares it four to one by weight, and one alone takes it all
  '';
  vm-boot-asks-in-the-theme = ''
    the splash asks for the disk, names a slow job, and lasts a restart to its end; a sign-in
    console greets the account, draws the theme's face, padded, signs in to a shell told it's
    kmscon, greets again once it's left, and its keys go back to the desktop; one kmscon can't draw
    falls back to the login prompt, greeting included
  '';
  vm-boot-asks-on-every-screen = ''
    with two screens of different sizes, the splash's ask shows whole on both, its bottom left
    corner on the smaller one included
  '';
  vm-checks-go-first = ''
    the daemon sits in a cgroup beside its builds, each build in one of its own handed cpu, io,
    memory and pids; a build's qemu goes from idle to normal at nice -10, every thread and a later
    one, and its build's cgroup is weighed at 1000 for CPU and disk, while a compiler's build beside
    it stays idle at 100; a build running a stand-in systemd-nspawn, with no qemu, is weighed at
    1000 and a service in it goes to normal at nice -10; a build asked for from a quest's slice
    takes the slice's weight and follows it; the daemon's own cgroup carries oomd's mark to pass it
    over; ten restarts of the daemon, vm-first looking every 20 ms, each start and leave the builds
    running, the daemon builds again, a build asking for a range of user ids gets 65,536, a
    uid-range build that makes a cgroup of its own is refused handing it pids with "Permission
    denied" and the daemon restarts beside it, a finished build that wrote 64 MiB is said with at
    least that and its client's command, one that wrote 64 MiB without waiting for the disk says at
    least that between what it wrote and what it left, a build's sync -f is refused with "Function
    not implemented" while its sync runs and root's sync -f outside the daemon works, a stopped
    quest's build is said with its quest, and a stopped build leaves no process nor cgroup
  '';
  vm-clock-keeps-time = ''
    the clock sets from the machine's servers past a network's silent one
  '';
  vm-commits-are-signed = ''
    commits are signed with the TPM's keys, PINs asked for
  '';
  vm-desk-facts-follow-a-replug = ''
    desk-facts names a USB sound card plugged into a running machine, stops naming it once it's
    pulled out, and names it again when it's plugged back in; names a card not on USB by each of
    its devices, saying what one plays at only while it plays; and names a USB keyboard with its
    polling rate until it's pulled out
  '';
  vm-desktop-captures-and-locks = ''
    the desktop takes a screen, a window, a region, a recording and the camera's pictures by the
    keys, and its lock screen, polkit's prompt and the askpass answer and hold
  '';
  vm-desktop-launcher-answers = ''
    the launcher lists each thing once for everything Joel types, copies what it answers, keeps
    emoji in their own place, and takes the weather's here from where whereabouts says the laptop
    is
  '';
  vm-desktop-launcher-goes-places = ''
    the launcher goes to pi's sessions, quests and runs and to windows, opens and moves them, sets
    the volume, loses no letter typed after its chord, and shows the shortcuts
  '';
  vm-desktop-opens-and-sets = ''
    the desktop keeps and gives back copies, opens pictures and documents, and sets sound, the
    power profile, displays and Wi-Fi from the launcher
  '';
  vm-desktop-says-what-happens = ''
    notices come, go and are acted on, and the margin, What's on, Tuple's calls and caps lock say
    what's happening and answer the keys
  '';
  vm-desktop-starts-clean = ''
    the desktop boots without an ordering cycle and its shell without an error, the meters answer
    the keys, programs wear the theme, and swipes and Bluetooth's agent work
  '';
  vm-dictation-hears-the-chord-let-go = ''
    an account outside the input group hears how long Super and D were held and a tap's, nothing of
    D alone, another key or Super after D, and can't read the keyboard itself
  '';
  vm-dictation-listens-once-it-hears = ''
    dictation's watchers hear it listening only once the microphone sounds, never for a press with
    no microphone
  '';
  vm-disk-guard-keeps-the-disk = ''
    the guards answer each band, sparing what's in use
  '';
  vm-first-finds-the-vms = ''
    vm-first reads a thread's nice and policy, knows a check with deadlines by its qemu, test
    driver, container or nested compositor, wrapped or cut short, and not a compiler nor a niri
    build's own test programs by their names, finds every process under a build and its
    containers' cgroups, knows the daemon from the workers it leaves behind, finds a worker's client
    and the quest's slice it sits in, keeps a quest's build below a VM's, adds up what a build read
    and wrote on every disk and what it left to write, and says a size, a client's command and a
    build's quest as a person reads them
  '';
  vm-first-passes-clippy = ''
    clippy finds nothing in vm-first or its tests
  '';
  vm-full-swap-ends-what-filled-it = ''
    full swap ends the cgroup that filled it through systemd-oomd, before the kernel's OOM killer,
    and leaves the rest
  '';
  vm-greeter-signs-in = ''
    the greeter signs in and out, not waiting on the account's setup
  '';
  vm-guide-installs = ''
    the guide, driven by its keys, holding the machine awake and drawing over what writes on its
    screen, installs a machine from its stick: two keys and a recovery key of seven words, shown
    alone for a phone and typed back, an install whose memory a restart took carried on through a
    key it enrolled, Secure Boot enforcing with its keys, its firmware enrolling them before the
    disk first asks; then seals the disk to the TPM with a PIN, which opens it at
    the next boot, leaving one systemd-pcrlock policy; makes the account's TPM keys under one PIN
    and a spare SSH key on each YubiKey, refusing one asked for by the other's name; clears the TPM,
    opens with a YubiKey, seals again, makes the keys it lost again and downloads a lost spare; a
    host writes a stick from the rescue bundle the install sealed to the recovery key, which the
    firmware boots while enforcing; then the install's own stick, which it made the machine's by
    carrying its keys and bundle and signing itself with them, boots enforcing and reinstalls the
    machine with the same keys, whose guide frees the first install's policy; and a firmware cleared
    of its keys gets them back from the machine's next boot, not while a program it would start is
    unsigned, and the TPM alone opens the disk once they're enrolled, the TPM and the YubiKeys
    sharing one PIN so no boot costs either a try
  '';
  vm-guide-opens-the-vault = ''
    a stick's vault sealed to a FIDO2 key opens with its PIN, and without it by the passphrase
  '';
  vm-guide-recovers = ''
    a repair stick finds a machine's disk, opens it with a YubiKey's PIN, mounts it whole, sets
    which generation boots next, replaces a lost key, reinstalls a boot loader gone from the ESP,
    and gives a root shell inside
  '';
  vm-guide-writes-a-stick = ''
    the guide writes a stick as the person, holding the host awake, through UDisks2, unmounting what
    was mounted from it, its passphrase seven words shown alone for a phone and typed back,
    another's words refused, the old stick's rescue bundles read from its vault mounted only for
    reading and the host's own kept for the person and carried onto the new stick, which is the
    host's own machine's, so the bundle it keeps for itself is opened with its passphrase and signs
    the stick's boot program with the machine's db key
  '';
  vm-ir-camera-opens-for-readers = ''
    the IR camera waits for udev and its loopback, opens only while read, its frames grey, no frame
    from before a reader opened
  '';
  vm-keyring-opens-without-asking = ''
    the keyring opens with its TPM-sealed password, unasked
  '';
  vm-libvirt-stops-quickly = ''
    libvirt's daemon stops at once, its udev thread starved
  '';
  vm-lid-stays-at-the-desk = ''
    a shut lid suspends with no monitor, keeps running with one plugged in whose screen is off,
    holding one lock through the card's changes, and suspends once it's unplugged
  '';
  vm-loopback-says-who-holds-it = ''
    a loopback camera's file names each program another account holds it with, not its relay's unit
    however a program names itself, one line a name, catches a holder that forks, empties when they
    close it or the camera stops, keeps its folder, follows a new loopback, and takes no loopback of
    another name
  '';
  vm-mv7-voice-is-levelled = ''
    an MV7's voice comes up 12 dB, its room and s sounds less, its peaks under -1 dBFS; another
    mic's are untouched
  '';
  vm-network-revives-units = ''
    what the network let down starts, and reloads on change
  '';
  vm-niri-is-traced-by-its-own = ''
    an account in wheel traces its own niri off the CPU with no password, each sleep said with its
    kernel stack and monotonic time and waits in epoll left out; nobody else starts it or reads it
  '';
  vm-passkeys-come-from-the-tpm = ''
    passkeys and security keys are made in the TPM, each after a yes
  '';
  vm-pebble-speakers-are-tuned = ''
    a Pebble X Plus hears its tuning, again once it's plugged back in, and headphones don't
  '';
  vm-plugging-in-wakes-it = ''
    every USB root hub may wake the machine, so a cable plugged into a sleeping laptop wakes it
  '';
  vm-reboot-cannot-hang = ''
    a reboot forces on after a minute of a service not stopping, the watchdog resets a stuck last
    stage, and Plymouth hears which
  '';
  vm-screens-come-and-go = ''
    the launcher, a notice, the margin's marks and dictation's bar come back after the screen goes
    and returns, after a console switch and after a sleep and wake, after which the shell still
    hears a notice, dictation's keys and Keep Awake let go; when one of two screens goes, the
    margin and dictation's bar stay on the other and the margin comes back on both; the launcher
    opens, finds and runs a row on either of two screens as each goes and comes back, as both go,
    and after a sleep, its foot still telling the time after the wake; Quickshell warns of nothing
    meanwhile; and desk-facts stops naming the screen once it's gone and names it again once it's
    back
  '';
  vm-secure-boot-installs = ''
    the whole install works, and the disk opens only for it
  '';
  vm-speakers-are-tuned = ''
    the XPS 14's speakers hear the tuning, headphones don't
  '';
  vm-swap-comes-down-while-probed = ''
    the compressed swap comes down, its reset waiting out udev's probe
  '';
  vm-the-desk-lights-come-back = ''
    a Litra plugged back in while a camera streams is told to light again, and the camera stopping
    puts it out; a Pebble X Plus plugged back in still has its lighting put out at a lock
  '';
  vm-the-line-stays-quick = ''
    under four uploads on an 8 Mbit/s line 20 ms away, an app's ping stays under 80 ms with the
    shaper and passes 300 without; an app's upload leaves the jobs' under a quarter of what it
    sends; the shaper follows the line down to 4 Mbit/s and back, and down again when the uploads
    ride a second link to the same router
  '';
  vm-the-network-keeps-its-order = ''
    an account's jobs, background, the system's builds and a guest it forwards for go in CAKE's bulk
    tin both ways, what Joel waits on at a terminal and an app's pages best effort, its calls, its
    UDP to Meet's media servers on 443 and Tuple's daemon video, even what it leaves to send as
    it ends, names and the shaper's pings voice; a call's packets turn real-time on; a slice made again keeps its class; the client's
    own network isn't slowed; inside a WireGuard tunnel the same classes reach the tunnel's own
    CAKE; the queues follow the default route, a link unplugged and plugged back in is shaped
    afresh both ways, udev sets nothing on the IFBs, and they go when the shaper stops
  '';
  vm-webcam-microphones-yield = ''
    a webcam's microphone ranks below a USB microphone's, again once it's plugged back in
  '';
  vm-wifi-joins = ''
    wifid joins, moves past a silent access point, nudges a stuck look, finds sign-ins, keeps
    others' routing rules, keeps the card awake on mains power
  '';
  what-vm-tests-share-does-what-it-says = ''
    the helpers VM tests share run a command as the account asked, with its runtime folder and
    bus, read a journal's cursor or refuse output without one, and pace each wait to how busy the
    host is as it starts, from as long as it says to four times that, describing the machine when
    one fails; a VM test written for pytest gets the driver's machines as fixtures, each test is a
    subtest named by its docstring, a failure fails it with pytest's account of why, and the
    driver ends as a failed subtest ends it
  '';
  whereabouts-passes-clippy = ''
    clippy finds nothing in whereabouts or its tests
  '';
  wifid-passes-clippy = ''
    clippy finds nothing in wifid, its sign-in window or their tests
  '';
  wifid-reads-the-air = ''
    wifid's tests pass: what it hears and offers, how each network is joined, the addresses it
    shows, what it keeps, and sign-ins
  '';
  wifid-sign-in-reads-its-network = ''
    the sign-in window's tests pass: its proxy lets on only the window, and it follows which card
    wants a sign-in
  '';
  without-syncfs-passes-clippy = ''
    clippy finds nothing in without-syncfs or its tests
  '';
  without-syncfs-refuses-only-syncfs = ''
    a program without-syncfs runs is answered "Function not implemented" by syncfs through a
    seccomp filter of its own while fsync runs, the filter refuses x32's syncfs too and lets every
    other call and another calling convention's through, and it says how it is used and what it
    could not run
  '';
  work-fills-swap-alone = ''
    the sorter gives each piece of work in jobs, awaited and timed work its own scope, so oomd ends
    only the one that filled swap, and pi's scopes share one slice weighted below a terminal's
  '';
  work-swaps-its-own-memory = ''
    jobs is slowed past half of memory, with no hard limit that would end its work
  '';
  work-yields-to-what-the-user-works-in = ''
    jobs is idle beside app.slice, pi finds it, and a window's scope is not idle
  '';
  writes-go-out-sooner = ''
    a machine with a screen keeps at most 128 KiB unsent in a socket, sends at full speed after
    idle, and asks for ECN with its fallback
  '';
  xdg-open-opens-each-kind-in-its-app = ''
    xdg-open opens markdown in the editor declared for text, in the terminal named for it, and a
    picture in its viewer
  '';
  yubikey-on-every-workstation = ''
    Linux and Mac workstations both have the YubiKey's tools
  '';
  zathura-accepts-its-configuration = ''
    zathura opens a page with the account's settings and stays open without a warning, and PDFs open
    in it
  '';
  zsh-is-the-login-shell = ''
    an account on NixOS logs in to zsh, as one on a Mac does
  '';
}
