# local-sites

The guide to the sites' server and the live sites on it, read before
changing either. What each file holds is in the [README][readme]; this
says how the pieces hold together and how to add a live page.

## The Platform

Every page Joel opens from this machine comes from one server,
`local-sites` ([`src/`][src]), on loopback at
`http://sites.localhost:8484/`. It serves three things:

- **Documents**: what sessions write into `~/.local/share/jitsusama/sites`.
  Each carries a copy of the web kit as a `kit/` of its own and takes its
  keys and theme switch from the shared kit's `document.js`. They're
  state, never a Nix value, and the home lists them from `sites.json`.
- **Live sites**: TypeScript on the [kit][kit], built by Nix with
  [`site.nix`][site]'s `localSite`, typechecked at its strictest, its tests
  run as it builds, and mounted at `/<name>/` from
  `jitsusama.local-sites.mounts`. Every live site reads the kit and the
  theme's web kit from the one copy at `/local-sites/`, so the browser
  fetches the fonts once for them all.
- **Feeds**: a WebSocket at `/feed/<feed>` the server bridges to one
  daemon's socket, as `jitsusama.local-sites.feeds` declares it, with the
  kinds and fields a page may read. The server is the daemon's client, as
  [`docs/daemons.md`][daemons] says, holding one watch for each kind of a
  feed that pages are reading and telling every tab from it. A watch no
  tab reads is let go when its daemon next answers, so a quiet daemon's
  stays open until then.

What holds it together:

- **A page reads and never acts.** A feed passes only reads (a watch, a
  list, a get, the daemon's health) and refuses the rest
  ([`src/gate.rs`][gate]). Records come trimmed to the kinds and fields
  declared; health comes as the daemon gives it. The server answers only
  its own names on loopback, and opens a feed only to a request naming a
  page of its own as its origin. A browser can't forge that, but any
  process on this machine can. Any page here, documents included, can
  read every feed and send what it reads wherever it can reach. So a feed
  declares only kinds and fields fit for any page and any process here,
  and it stays safe because it never acts.
- **The build writes the strip**, straight after a page's skip link, and
  refuses a page that carries its own, reads no theme from the shared
  copy, names a `kit/` of its own, or has other than one skip link
  ([`site.sh`][sitesh]).
- **A file is answered by its tag.** The server sends an ETag with every
  file and answers a browser that already holds it with a 304, so a
  second visit sends no body again.
- **No script or sheet a site builds is sent with its comments.** The
  kit's tsconfig drops them from every site's TypeScript;
  [`site.sh`][sitesh] sends a script written by hand through the same
  compiler and each sheet through esbuild, unminified, so rules and
  declarations keep their order; the kit's `switch.css` and the theme's
  `base.css` go the same way. A script or a sheet still holding a
  comment marked to stay fails its build, the kit's too
  ([`kept-comments.sh`][kept], [`bare-sheet.sh`][baresheet]). The
  comments stay in the tree. Still sent
  with theirs: `theme.css`, which the home reads as it is written, the
  home's own `strip.css` and `home.css`, which home.py copies, and the
  pages' few HTML comments, the strip's markers among them.
- **A live page with a reads-live test holds a budget**: today all six,
  the system page, the map, murmurs, Code, the network and forage. The
  test runs in headless Chromium through the real
  server beside a stand-in daemon. It holds a first visit to a number of
  files, kilobytes and elements, and a second to every file answered
  unchanged. A change worth more raises the budget in the same change.
  The failure says which measure passed its budget, and for elements,
  which tags grew.

## Adding a Live Page

1. **Write it** two folders below `modules/`, as `modules/system/site/`
   or `modules/mycelium/murmurs/`, so its imports of
   `../../local-sites/kit/` resolve in the tree as they do in the build.
   It's served one folder deep, at `/<name>/` for the name it's mounted
   under, which needn't be its folder's, where the same address lands on
   the kit's mount. The site is flat: only its top-level `.html`, `.css`,
   `.svg`, `.md`, `.txt` and `.js` files, and the modules its TypeScript
   compiles to, are shipped. Give it:
   - an `index.html` whose head follows the system page's: the `theme-color`
     lines on `@dark-ground@` and `@light-ground@`, the script that
     restores the theme, `favicon.svg`, the web kit's font preloads, then
     `../local-sites/web/theme.css` (the build refuses a page without it),
     `base.css` and the kit's `switch.css` before its own styles, and one
     skip link;
   - a `favicon.svg` drawn in `@dark-tint@` and `@light-tint@`, which the
     build fills with its tint, as it fills the grounds;
   - its TypeScript on the kit's `feed.ts`, `records.ts` and `status.ts`,
     a `tsconfig.json` like the system page's, and any `tests/*.test.ts`.
     A command it offers to copy is built from feed data and goes into a
     shell, so quote every value in it.
2. **Declare it** in [`home.nix`][module]: its mount, as
   `sites.localSite { name; src; inherit webKit; tint; }`, where `tint` is
   the theme role its emphasis wears; the kinds and fields it reads under
   `jitsusama.local-sites.feeds`, in the daemon's feed if one is already
   declared, since a watch is shared only within one feed; and its row
   under `jitsusama.local-sites.rows`, which the home lists in place of any
   row `sites.json` has at its path. Put it behind what its daemon needs
   (forage behind a mycelium realm, Code behind repository roots), so a
   machine without the daemon neither mounts nor lists it. A page whose
   daemon a machine may lack, as the network page is mounted everywhere,
   says it isn't live and why, as the kit's `status.ts` words it, rather
   than failing.
3. **Prove it** in [`checks.nix`][checks]: `<page>-site-holds`, named for
   the page rather than its mount (murmurs, mounted as `web`, has
   `murmurs-site-holds`), the site built on plain looks, and, inside
   `onLinux`, a reads-live test kept in the site's own `tests/`, on
   [`tests/live_page.py`][livepage] beside a stand-in for its daemon. Pass
   it the feed's declared kinds, as the map and murmurs pass `NODE_KINDS`,
   so the server trims as it will here and a field the feed drops fails
   the test; the system page's test still writes its own and isn't the
   one to copy for this. It holds the web-design skill's page checks and
   a `Budget` set a little above what the page costs now. Name each check
   in `check-descriptions.nix`.

A live site can also leave this machine as one folder that stands alone:
`site.nix`'s `standalone`. It refuses a folder whose pages, styles or
modules still load from outside it, by what it can read, and its header
says what it can't: an address handed to fetch by a name, as Code reads
its rows, is one. The page still asks its feeds at `/feed/` on the host
it's served from, and draws what answers there, the commands it offers to
copy into a terminal among them, so a folder belongs only on a host whose
feed the reader trusts. Deploying it stays outside core.

[baresheet]: ../theme/web/bare-sheet.sh
[checks]: ./checks.nix
[daemons]: ../../docs/daemons.md
[gate]: ./src/gate.rs
[kept]: ./kept-comments.sh
[kit]: ./kit
[livepage]: ./tests/live_page.py
[module]: ./home.nix
[readme]: ./README.md
[site]: ./site.nix
[sitesh]: ./site.sh
[src]: ./src
