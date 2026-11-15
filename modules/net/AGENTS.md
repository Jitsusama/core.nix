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
access point, switch or client (docs/glossary.md's Network words). One
eero is one device with the router and access point roles, listed once.
Each vendor is a source behind those roles: eero's record (the eero
daemon's), the cable modem's status pages and the switch's web page.
No vendor's command reads another vendor's equipment, so the modem and
the switch are net's alone, and `eero` keeps only what's eero's.

## The Record

[`tests/network.json`][record] is the home network as one record,
`net.network/1`, the shape the daemon writes to
`$XDG_RUNTIME_DIR/net/network.json` and every view reads:

- `collected_at`, and each source's `read_at` and `since`: seconds since
  the epoch, so a reading's age is one subtraction.
- `sources`: each source, whether it's answering, unreachable or signed
  out, and why.
- `devices`: each device's generic fields, its ports and radios, a
  role's own block (a modem's channels and log), and `detail`, every
  field each source gives about it in the source's own words.
- `links`: a cable or a radio link, port to port or radio to radio; the
  map is drawn from these.
- `line`: the internet provider's line beyond the modem, from eero's last
  speed test, which runs from the gateway and so measures the line alone.
- Each device's `path` and the record's `concerns`, worked out rather
  than read (`src/room.rs`). A path lists the devices to the modem and,
  each way, its chokepoint: the hop with least room, a cable's speed or a
  radio link's less the time its busier radio is busy, or the line. A hop
  with no figure (eero gives Wi-Fi backhaul only as bars) is counted in
  `unmeasured`, never guessed. A concern is a radio busy 30% or more (the
  map's gold), a Wi-Fi link at -70 dBm or weaker, a cable under a gigabit,
  or the line as a chokepoint, said once each way.

A figure keeps its unit in its key (`speed_mbps`, `power_dbmv`, `up_s`),
and one a source doesn't give is `null`, never a guess. `src/model.rs`
holds the types, and its tests hold the record to them both ways.

## The Daemon

`net daemon`, the user service `net-daemon` in background.slice, reads
each source once a minute and writes the record whole, through
`network.json.new` and a rename, so a view never reads half of one.

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
drops a reply when asked for two, and asks for a page again once when it
answers nothing. It signs in only when the pages come back holding no
status, as they do once the switch's session has ended, because the
sign-in is plain HTTP and puts the password on the home network in the
clear. It writes `$RUNTIME_DIRECTORY/report.json` with only what the pages
show: the password, the sign-in and ureq's own error words never go in it.
With no credential the report says signed out, and nothing else changes.

## The Views

Every view reads the record alone, never a source, so it answers at once
and the same way away from home. `src/views.rs` draws each as a function
of the record, the time, a pen and the room, so its tests set every
width without a terminal; tables and facts come from the terminal kit.

- `net`: how each source answers and how long ago, how many devices do
  what, and the commands that go further.
- `net devices`: one line per device, its roles, address, time up and
  model; a narrow terminal drops the model, then the time, then the
  address.
- `net devices NAME` (`device` too): one device in layers. What any
  device can say, its ports and radios, the links it's on, a modem's
  channels and log, then what each source says that those didn't, in
  the source's words. A list of records or a series of readings is left
  to `--json`, which keeps every source's detail whole.

A name is matched in any case, and an id always works. eero can give
two devices one name, so a shared name is refused (64) with each one's
id; an unknown one is refused (66) with the nearest. `--json` answers
`net.network/1`, `net.devices/1` or `net.device/1`, each time in RFC
3339 (docs/commands.md). No record yet ends 69, pointing at
`systemctl --user status net-daemon`.

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
[contract]: contract/README.md
