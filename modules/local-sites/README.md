# Local Sites

Joel's local sites, served on this machine alone at
`http://sites.localhost:8484/`: the documents sessions write under
`~/.local/share/jitsusama/sites`, the home built from their rows
([`home/`][home]), and the live sites, each page fed by the daemon that
holds its data.

The server is a Rust program, `local-sites` ([`src/main.rs`][main] says
how). A page reads a feed at `/feed/<name>`, a WebSocket over which it
speaks the daemon interface every daemon here speaks
([`docs/daemons.md`][daemons]). The server holds one watch on the daemon
for each kind any page reads, and tells every page and tab from it. A
page reads and never acts; it sees only the kinds and fields its feed
declares in `jitsusama.local-sites.feeds`. The server answers only its own
names on loopback, and opens a feed only to its own pages.

A live site is TypeScript on the kit, built by Nix: [`site.nix`][site]
typechecks it at its strictest, runs its tests and compiles it beside its
page, so a site that doesn't hold never reaches the server. A site's
source sits at `modules/<name>/site/`, served at `/<name>/`, and
[`home.nix`][module] mounts it with the feed it watches. The system page
([`modules/system/site`][system]) is the first. The forage page
([`modules/mycelium/forage`][forage]), served at `/forage/`, sits
beside the mycelium module it reads, since `/mycelium/` is the map's.

| Path                     | What it holds                                           |
| ------------------------ | ------------------------------------------------------- |
| [`home.nix`][module]     | the server's unit, its options, the sites and the home  |
| [`package.nix`][package] | the server's build, its tests run as it builds          |
| [`site.nix`][site]       | how the kit and a live site are built and tested        |
| [`src/`][src]            | the server: config, door, files, gate, feeds, pages     |
| [`tests/`][tests]        | the server run beside a stand-in daemon, and scripts'   |
| [`kit/`][kit]            | what every live site shares: feeds, records, the switch |
| [`home/`][home]          | the home page's builder and its scripts and styles      |

The mycelium map's timer (`mycelium-map.sh`, `mycelium-map.jq`) and the
network site's copy of net's record stay until each site reads its feed.

[daemons]: ../../docs/daemons.md
[forage]: ../mycelium/forage
[home]: ./home
[kit]: ./kit
[main]: ./src/main.rs
[module]: ./home.nix
[package]: ./package.nix
[site]: ./site.nix
[src]: ./src
[system]: ../system/site
[tests]: ./tests
