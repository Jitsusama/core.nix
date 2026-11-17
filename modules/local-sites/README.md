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
([`modules/system/site`][system]) is the first, and the mycelium map
([`modules/mycelium/site`][map]) is fed live by the node. Code
([`modules/repos/site`][code]) draws every repository live from the repos
daemon; where each repository and tree is home on it changes with the
disk, so `code-site.sh` writes its `homes.json` into the documents every
minute, and a mount without its own `homes.json` answers with the
documents' one under its name. The forage page
([`modules/mycelium/forage`][forage]), served at `/forage/`, sits
beside the mycelium module it reads, since `/mycelium/` is the map's, and
so does the murmurs page ([`modules/mycelium/murmurs`][murmurs]), served at
`/web/`, where the home has always listed it.

The build also writes the strip every one of Joel's sites opens with,
straight after the page's skip link: the way home and that the page is an
app, nothing more, since the page's own line says how live it is. A page
carries no strip of its own; one that does fails its build. The home
writes the same strip into the documents it may change, so both read
alike.

| Path                     | What it holds                                           |
| ------------------------ | ------------------------------------------------------- |
| [`home.nix`][module]     | the server's unit, its options, the sites and the home  |
| [`package.nix`][package] | the server's build, its tests run as it builds          |
| [`site.nix`][site]       | how the kit and a live site are built and tested        |
| [`src/`][src]            | the server: config, door, files, gate, feeds, pages     |
| [`tests/`][tests]        | the server run beside a stand-in daemon, and scripts'   |
| [`kit/`][kit]            | what every live site shares: feeds, records, the switch |
| [`home/`][home]          | the home page's builder and its scripts and styles      |

A live page's own test reads it in headless Chromium as Joel's browser
would, through the real server beside a stand-in for its daemon. What
every such test shares lives once in `tests/`: the stand-in
(`stand_in_daemon.py`) and the server, the browser, a tab and the
web-design skill's page checks (`live_page.py`, loaded as a pytest plugin
for its `browser`). The system page's is `system-site-reads-live`, and the
map's `mycelium-site-reads-live`.

[code]: ../repos/site
[daemons]: ../../docs/daemons.md
[forage]: ../mycelium/forage
[map]: ../mycelium/site
[murmurs]: ../mycelium/murmurs
[home]: ./home
[kit]: ./kit
[main]: ./src/main.rs
[module]: ./home.nix
[package]: ./package.nix
[site]: ./site.nix
[src]: ./src
[system]: ../system/site
[tests]: ./tests
