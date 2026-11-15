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

It listens on nothing and asks nothing outside the home network. Away
from home the modem doesn't answer, and the record says so while eero's
devices stay.

## Changing It

[`contract/modem/`][contract] holds a real modem's pages, its addresses
and serials replaced, and `src/collect.rs`'s tests collect them with
eero's golden record into the golden record. After a change that's
meant to move it, `NET_BLESS=1 cargo test` writes it anew; read the
diff before committing it. `cargo` runs from the dev shell:
`direnv exec ../.. cargo test` here.

[record]: tests/network.json
[contract]: contract/modem/
