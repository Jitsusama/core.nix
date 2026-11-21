# net

The whole home network, read from this machine: every device on it, the
roles each plays, and the links between them. A Rust program, `net`,
keeps one record of it and draws it for Joel. His bar (2026-11-14):
"i want fully nerdiness for switch and modem details, at least to the
level of detail for eero", in generic words that fit any device, "i
never want to lose details just to make them fit into a box", and maps
derived from what's read.

## The Words

A device is one box, and its roles are what it does: modem, router,
access point, switch, receiver, printer, television or client
(docs/glossary.md's Network words). One eero is one device with the
router and access point roles, listed once. Each vendor is a source
behind those roles: eero's record (the eero daemon's), the cable modem's
status pages, the switch's web page, a Sony receiver's UPnP answers, an
HP printer's status pages, an Apple TV's AirPlay answer and a Samsung
TV's own page. No vendor's command reads another vendor's equipment, so
the modem, the switch, the receiver, the printer, the Apple TV and the
TV are net's alone, and `eero` keeps only what's eero's.

## The Record

[`tests/network.json`][record] is the home network as one record,
`net.network/1`, the shape the daemon writes to
`$XDG_RUNTIME_DIR/net/network.json` and every view reads:

- `collected_at`, and each source's `read_at` and `since`: seconds since
  the epoch, so a reading's age is one subtraction.
- `sources`: each source, whether it's answering, unreachable or signed
  out, and why. A source reads only the minute it's in, so the daemon
  carries each over from the last minute that named it in the last five,
  the record an earlier daemon wrote included: one still not answering in
  the same way keeps when that began, and one that didn't answer keeps
  when it last did. So a minute eero doesn't answer, naming none of its
  devices, loses nothing.
- `devices`: each device's generic fields, its ports and radios, a
  role's own block (a modem's channels and log, a receiver's volume and
  what it's doing, a printer's ink and what it's doing, a television's
  power), a client's `speed`
  each way as eero counts it, now and at its busiest in eero's last hour
  with when, and `detail`, every
  field each source gives about it in the source's own words.
- `links`: a cable or a radio link, port to port or radio to radio; the
  map is drawn from these.
- `internet`: the internet beyond the modem, its provider and speed each
  way from eero's last speed test, which runs from the gateway and so
  measures the internet alone; and `carried`, what the line carried over
  the last day as eero counted it at the gateway, the bytes each way and
  each hour's average speed, so an hour reads against the line's own.
- Each device's `path` and the record's `concerns`, worked out rather
  than read (`src/room.rs`). A path lists the devices to the modem and,
  each way, its chokepoint: the hop with least room, a cable's speed or a
  radio link's less the time its busier radio is busy, or the internet. A hop
  with no figure (eero gives Wi-Fi backhaul only as bars) is counted in
  `unmeasured`, never guessed. A concern is a radio busy 50% or more (the
  map's gold), a Wi-Fi link at -70 dBm or weaker, a cable under a gigabit,
  the internet as a chokepoint, said once each way, a modem channel that
  lost 100 or more it couldn't repair and ten times any other's, unless
  the day shows it lost none lately, or a
  device using 80% or more of the internet's speed one way, which fills
  it for every other. They're listed worst first, by how far each kind
  reaches: the channel's losses, a device filling the line, a slow cable,
  a busy radio, a weak link, then the internet.
- What the day says, from net's own readings (`src/history.rs`): each
  radio's `busiest_pct` and `busiest_at`, and the modem's `lately_s` with
  each downstream channel's `lost_lately`, counted from the earliest
  reading since the modem last started and only once that reaches back an
  hour. The busy and lost concerns say them when they're known.

A figure keeps its unit in its key (`speed_mbps`, `power_dbmv`, `up_s`),
and one a source doesn't give is `null`, never a guess. The day's figures
are left out until net has readings for them, so a record without a day
reads as it did before them. `src/model.rs`
holds the types, and its tests hold the record to them both ways.

## The Daemon

`net daemon`, the user service `net` in background.slice, reads
each source once a minute and writes the record whole, through
`network.json.new` and a rename, so a view never reads half of one.
It serves the same record on `$XDG_RUNTIME_DIR/daemons/net.sock`, which
systemd holds for it, in the daemon interface every daemon speaks
([docs/daemons.md][daemons]), through daemon-serve. Each round hands
daemon-serve the record split by kind ([`src/serve.rs`][serve]), so a
watch is told only what moved:

- `device:<id>`: a device whole, its roles, radios, ports, speed and path.
- `link:<a>|<b>`, its ends sorted: a link whole, with `crossings`, how
  many devices' routes to the internet cross it. It counts routes, not
  traffic, since eero gives no figure for what a link carries.
- `source:<name>`: how each vendor's source stands, with its reason.
- `internet`: the one record of the internet beyond the modem.
- `concern:<concern>:<device>[:<on>][:<other>]`: what's worth a look, its
  own kind as `concern`, since `kind` is the record's, and its `rank`, its
  place worst first from 0, since a watch hears records in key order.
- `uplink`: who fills this machine's own upload, served only while it's
  watched, and only on Linux ([The Uplink](#the-uplink)).

The record's `collected_at` isn't served: a device and a source carry
their own `read_at`, and the internet its `measured_at`. Health says each
source's state, with its reason when it isn't answering.
Each minute it also appends the figures that swing, each channel's losses
and each radio's busy share, to `$XDG_STATE_HOME/net/day.jsonl`, and
reads the last day of them back before writing the record. The file is
trimmed to the day about once an hour, through `day.jsonl.new` and a
rename, and a line that doesn't parse is skipped, so a crash costs a
minute. It survives a restart, and a file that can't be kept costs only
what net says of the day.

- eero: the eero daemon's record, `$XDG_RUNTIME_DIR/eero/home-network.json`,
  never eero's cloud, which keeps one reader. `src/sources/eero.rs` maps
  each node and device; its whole entry stays as `detail.eero`.
- The modem: nine status pages at `http://192.168.100.1/data/<page>.asp`,
  read without signing in, one at a time with a two-second limit.
  `src/sources/modem.rs` maps them, and every page's answer stays as
  `detail.modem`. Its log's times are placed by the modem's own clock
  against ours, rounded to 15 minutes, and `clock` keeps what it wrote.
- The switch: its reader's report, `/run/net-switch/report.json`
  (`net.switch/1`), never the switch itself (below). `src/sources/switch.rs`
  maps it into one device with the switch role, which stands for the box
  where eero lists it as a wired client by the same hardware address; the
  report's status stays whole as `detail.switch`. When one port alone is
  linked, eero's cable to the switch comes in on it.
- A Sony receiver: each device eero lists with Sony as its maker and
  audio as its kind, at the address eero gives. It asks its UPnP
  description at `:8080/description.xml`, then, posted to the services
  the description names, `GetVolume`, `GetMute`, `GetTransportInfo`,
  `GetMediaInfo` and `GetPositionInfo`, then its rendering service's
  description for its volume's scale, then Sony's action list and
  `getSystemInformation` from it: nine reads, one at a time, two seconds
  each. A URL an answer names is asked only when it's on the receiver's
  own address, and no redirect is followed, so no answer can send a
  request outside the home. `src/sources/sony.rs` puts its volume and
  what it's doing, in plain words, on eero's device for it, now a
  receiver, and every answer stays as
  `detail.sony`, read from XML by `src/xml.rs`. Registering as its
  remote, sending it a key or text, or setting its volume waits on
  Joel's yes, so none is asked.
- An HP printer: each device eero lists with HP or Hewlett as its
  maker and printer as its kind, at the address eero gives. It asks four
  fixed LEDM pages under `/DevMgmt/`, `ProductStatusDyn`,
  `ConsumableConfigDyn`, `ProductConfigDyn` and `ProductUsageDyn`, one at
  a time, two seconds each, following no redirect and nothing an answer
  names. `src/sources/hp.rs` puts what it's doing, its ink and the pages
  it has printed on eero's device for it, now a printer, with its model,
  firmware and serial, and every page stays whole as `detail.hp`.
  Printing, scanning, changing a setting or clearing an alert waits on
  Joel's yes, so none is asked.
- An Apple TV: each device eero lists with Apple as its maker and
  media streamer as its kind, at the address eero gives. It asks
  `:7000/info` once, two seconds at most, following no redirect: what
  the AirPlay receiver says of itself to anyone, as a binary property
  list. `src/sources/airplay.rs` gives eero's device for it its model by
  the name Apple sells it under (`AppleTV6,2` is an Apple TV 4K, 1st
  generation) and its build as firmware; it stays a client, since no
  role names a box that plays to a screen. The answer stays as
  `detail.airplay`, but for the keys it pairs with and the address it
  was asked from. What it's playing and its volume are said only to a
  paired device, and pairing waits on Joel's yes, so none is asked.
- A Samsung TV: each device eero lists with Samsung as its maker and
  generic or media streamer as its kind, at the address eero gives, so
  a box eero places as a phone, tablet or computer isn't asked. It asks
  `:8001/api/v2/` once, two seconds and 64 KB at most, following no
  redirect: what the TV says of itself to anyone, as JSON. Only an
  answer saying `Samsung SmartTV` counts; any other box says nothing,
  answering or not. `src/sources/samsung.rs` makes eero's device for it
  a television, with its model, whether it's on or in standby, and the
  name it was given on its own screen, which stands for eero's unless
  Joel nicknamed it there. A TV heard once is remembered while the
  daemon runs: a minute it's silent, or answers as something else, keeps
  its name, role and model and says it isn't answering as a TV. A TV
  asleep stays on eero's list but goes silent, so silence leaves the
  source answering, with the time it last answered; only an answer that isn't a
  TV's (another page, an error status, a redirect or a page past 64 KB),
  while no TV answers as one, makes it unreachable. A TV switched off leaves eero's list, so
  it isn't asked, and is forgotten. The answer stays as
  `detail.samsung`, but for its ids and the addresses eero gives. Sending
  it a key waits on Joel pressing Allow on the TV, so none is sent.

It listens on nothing and asks nothing outside the home network. Away
from home the modem doesn't answer, and the record says so while eero's
devices stay.

## The Switch's Reader

A TP-Link Easy Smart switch shows its ports only once signed in, and its
admin password lets its holder mirror ports or change VLANs, so it never
reaches Joel's account, where every agent runs. `net switch-reader
ADDRESS` (`src/switch_reader.rs`) runs apart, as the system service
`net-switch` (`nixos.nix`) under a user of its own, on a machine that sets
`jitsusama.net.switch.address`. It holds the password as the encrypted
credential `/etc/credstore.encrypted/switch-password`, written by hand with
`systemd-creds encrypt`, and reads it as `$CREDENTIALS_DIRECTORY/switch-password`;
it may reach the switch's address and nothing else.
Each minute it reads four status pages one at a time, since the switch
drops a reply when asked for two, and asks for a page up to five times
when it answers nothing, the pause doubling from 300 ms, since its drops
come in bursts. System Info is read first, and the read stops there when
it holds no status, since a signed-out switch may hang up on the other
pages rather than serve its sign-in form. It signs in only then, once
the switch's session has ended, because the sign-in is plain HTTP and
puts the password on the home network in the clear. Each change of what
the switch reads, and why, goes to the service's journal. It writes
`$RUNTIME_DIRECTORY/report.json` with only what the pages show: the
password, the sign-in and ureq's own error words never go in it.
With no credential the report says signed out, and nothing else changes.

## The Views

Every view reads the record alone, never a source, so it answers at once
and the same way away from home. `src/views/` draws each as a function
of the record, the time, a pen and the room, so its tests set every
width without a terminal; tables and facts come from the terminal kit.
A device's page is `device.rs`, what its sources say in their own
words is `said.rs`, and what the line carried over the last day is
`carried.rs`.

- `net`: how each source answers and how long ago, how many devices do
  what, each concern but the ceilings in net's words beside its kind
  under Worth a Look, the ceilings under The Internet, since they're the
  line's speed rather than a fault, then what the line carried over the
  last day each way: its bytes, average, share of the line's speed and
  busiest hour, and the commands that go further.
- `net devices`: one line per device, its roles, address, time up and
  model; a narrow terminal drops the model, then the time, then the
  address.
- `net devices NAME` (`device` too): one device in layers. What any
  device can say, its ports and radios, the links it's on, a modem's
  channels and log, a receiver's volume and what it's doing, a
  printer's ink, what it's doing and the pages it has printed, a
  television's power, or that it isn't answering as a TV, then what
  each source says that those didn't, in the source's words. A list of
  records or a series of readings is left to `--json`, which keeps every
  source's detail whole, but for a printer's statuses, read field by
  field as HP names them; its usage counters and settings are left there
  too, as is what describes a maker's file rather than the device (HP's
  versions, schemas and alert counter). A source's section says each
  path once: its lone facts first, then each group of facts under a line
  of the path they share, two cells in, and so again at every depth.

A name is matched in any case, and any of a device's addresses or its
id always works. eero can give two devices one name, so a shared name
is refused (64) with each one's address and access point, offering the
first one's address to type; where devices share an address too, each
one's id follows and is what's offered. An unknown one is refused (66)
with the nearest. `--json` answers
`net.network/1`, `net.devices/1` or `net.device/1`, each time in RFC
3339 (docs/commands.md). No record yet ends 69, pointing at
`systemctl --user status net`.

## The Uplink

`net uplink` says who filled this machine's own uplink over the last
5 s, which no device's record can: Joel asked on 2026-11-15 why so much
goes up, and the answer was by process and destination, which only this
machine's sockets hold. `src/uplink/` reads every TCP socket and what it
has sent from the kernel's socket diagnostics over netlink (`diag.rs`),
not by running `ss`, since a program per sample is the slow way and the
daemon will sample often. It finds each socket's process through
`/proc/*/fd`, once per new socket (`owners.rs`), and tells two things
apart (`purpose.rs`): what the bytes are for (agent sessions' model
requests, builder tunnels, image pushes, backups, git, other) from the
process and its cgroup, and whose they are, Joel's apps or the agents'
slices, from where systemd placed it. A peer on this network never
reaches the uplink, so it's left out. What the sockets don't hold, UDP
such as QUIC, headers and a VPN's own wrapping, is said as unattributed
rather than spread over the rest, and another user's sockets, root's
included, have no owner to read and say so. `--json` answers
`net.uplink/1`, with Joel's share and the agents' to the nearest 5% for
a glance to show.

The daemon serves the same report on net.sock as the sole `uplink`
record, on Linux only. It's read every 5 s on a thread of its own, and
only while a watch asks for the uplink, by its kind or its key, since
reading every socket's owner costs more than a record nobody shows is
worth. When the last such watch ends, the record goes, so the next
watch never hears an old one. `Served` in `src/serve.rs` holds the
network's records and the uplink's together, since daemon-serve is
handed every record at once and drops any left out.

## Changing It

[`contract/`][contract] holds a real modem's pages and a real switch's,
their addresses and serials replaced, and `src/collect.rs`'s tests collect
them with eero's golden record into the golden record. The reader's tests
run it against a stand-in switch on a loopback port, which serves the
contract's pages only once signed in. After a change that's
meant to move it, `NET_BLESS=1 cargo test` writes it anew; read the
diff before committing it. `cargo` runs from the dev shell:
`direnv exec ../.. cargo test` here.

[record]: tests/network.json
[daemons]: ../../docs/daemons.md
[serve]: src/serve.rs
[contract]: contract/README.md
