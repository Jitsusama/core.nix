# Where Sites Live, and the Kit

The operational side of a site: where it is served, the record that lists
it, how one site links to another's things, and the kit of theme CSS and
fonts. Load this when making, moving or publishing a site, or wiring its
links home. None of it is design: the kit gives a site its colours and faces
so it never retypes a value, and the site still makes its own world on top
(`language.md`, A World of Its Own).

## Where a Site Lives

A site lives where its readers are. One shared with other people goes on
the host they reach. Anything just for Joel (a mock, a tour, a record, a
page written for him) lives on his own machine, under one local home, never
on a shared host: "we need to not creeeeeate [shared] sites for stuff like
luancher shapes ... we can run them locally though as just standard sites,
that is awesome" (2026-11-11).

- **One home ties them together.** Each site is a folder under
  `~/.local/share/jitsusama/sites/`, reached at
  `http://sites.localhost:8484/<site>/`, and the home page at its root lists
  every site, newest first, each saying what it is and what it asks of him.
  A site with no value left is removed, row and folder.
- **One record, never the home by hand.** The home and its `llms.txt` are
  built from `sites.json` at the root, a row per site: its path, name, what
  it is in a sentence, what it asks of him (or nothing), its date, its kind
  (mock, record, explainer, test or app), its status (growing, settled or
  withdrawn), the quest it came from, the kinds of thing it's home to, if
  any (One Home for Every Thing, below), and `pages`, its own pages whose
  strip says another kind (a test page beside a record). A new site adds
  its row there, and that's all: core's local-sites module rebuilds the
  home, its `llms.txt` and every page's strip within a second of the
  record changing. Write the whole record to a file beside it and rename
  it over, so the builder never reads it half written; a record it can't
  read keeps the last build. A turned-down or settled site moves to the
  home's Kept for Reference, below the rest.
- **A live site's row is declared with it.** For a live site (system,
  network, forage, mycelium, web or code, on a machine that serves it),
  what the home says of it comes from `jitsusama.local-sites.rows` in
  core's `modules/local-sites/home.nix`, beside its mount, and stands in
  for its row in `sites.json`. Its words change there, as a change to
  core, and reach the home at the next switch; an edit to its
  `sites.json` row no longer reaches the home. The home writes every row
  it lists, the declared ones as declared, to `rows.json` beside
  `sites.json`. The system's and Code's readers of the homes read
  `rows.json`, and `sites.json` only where a home has none yet;
  `homes.js` reads `rows.json` too, and checks `sites.json` beside it.
  So a change to a live site's homes is made in `home.nix` alone. Leave
  its `sites.json` row as it is: a home from before `rows.json` still
  reads it. A document's row in `sites.json` is still
  the whole job.
- **A document on the kit.** A document takes its keys and its theme
  switch from the kit, never a script file of its own (the lines in its
  head that set the kept look before the first paint stay): it loads
  `<link rel="stylesheet" href="/local-sites/kit/switch.css">` and
  `<script type="module" src="/local-sites/kit/document.js"></script>`,
  and no `site.js`. Its parts are `.part` sections, each opening with an
  h2; its list of keys is the dialog `#keys`; j, k and ? where its header
  shows them, `header .keys kbd`, can be pressed as well as typed; and its
  switch is the kit's radios (`craft.md`, Dark, Light and the Switch). So
  every document learns what the kit does next, and the home names each
  one still loading a script of its own as it builds.
- **Every page has a way home.** Each page opens with one quiet strip,
  `<nav class="sites-strip">` from `base.css`: All your sites, linking the
  home, then the page's kind, as of when, and what it asks of him. Built
  from the same row, so it never disagrees with the home. "if i can't
  access something, i don't use it" (2026-11-11).
- **Nothing is a one-off.** A site for him is never put anywhere else, and a
  shared one that's done with is taken down rather than left to drift.
- **An app's source lives in a repository.** The home's folder is what's
  served, not where code is kept. An app he relies on (a live map, a page
  that reads a program's record) keeps its source beside the program that
  feeds it, in that program's repository, with its probes as its gate, and
  is installed into its folder from there. A live site in core is
  TypeScript on the local-sites kit at `modules/<name>/site/`, built and
  tested by Nix and fed by its daemon (`modules/local-sites/README.md`).
  A dated record or a mock is written once and stays in the folder as it
  was written, with no core change, so a quick one stays quick.

## One Home for Every Thing

The sites are one place, not a shelf of separate ones. Every thing they
show has one home: one view, on one site, at one address, that is about it.
Every other mention of it, on any site, links to that home, and every home
links down to what the thing holds. So Joel can start anywhere and follow a
thing to what it's made of and what it touches. He asked for it twice on 15
Nov: Code reaching the mycelium, a machine reaching its cells and what it's
using, a repository reaching the cells at work in it, and the network
reaching its machines. Without one convention, each site would invent its
own links, and none would meet.

- **One site is home to each kind.** A new kind gets its row here before any
  site shows it, and moving a kind's home is a change to this table:

  | Kind       | Home        | Its address, for example           |
  | ---------- | ----------- | ---------------------------------- |
  | machine    | `system/`   | `system/#machine=studio`           |
  | cell       | `mycelium/` | `mycelium/#cell=E2ZIAQ`            |
  | network    | `mycelium/` | `mycelium/#net=repo.world`         |
  | repository | `code/`     | `code/#repo=Jitsusama%2Fcore.nix`  |
  | tree       | `code/`     | `code/#tree=<its path, encoded>`   |
  | device     | `network/`  | `network/#device=kitchen-printer`  |

- **An address is `<site>/#<kind>=<key>`**, the key URI-encoded. It's a
  fragment rather than a query, since the server is static and a fragment
  lands without a reload. The key is the thing's steady name: a node's
  name, a cell's address, a repository's owner and name, a tree's path, a
  device's slug. An id may sit in an address, and never on the page.
  Landing on one puts focus and the ring on the thing and opens its view,
  as stepping into it would, and Back returns to where he was. An address
  that was once a home keeps landing, so an old link never breaks.
- **A home site says what it holds.** It writes `homes.json` beside its
  index, every thing it's home to as `{"kind", "key", "name", "names",
  "address"}`, where `names` holds any other name a page may call it by
  (a repository's short name, a device's) and `address` is from the
  sites' root, the site's own path and then a fragment (`#`), in plain
  visible ASCII with no double quote, backslash, angle bracket or
  backtick. A record whose address is anything else is dropped. A live site writes
  it with its data, so it's never stale. Its row in `sites.json` names its
  kinds, `"homes": ["machine", "cell", "network"]`. A kind two rows
  claim is a fault in both.
- **A pattern homes a whole kind.** Where every thing of a kind has its
  page by construction, the row says so instead of listing them:
  `{"kind": "machine", "address": "#machine={name}"}` beside or in place
  of its kinds. The address is a fragment on the site's own page, written
  from the page (`#machine={name}`) or from the sites' root
  (`system/#machine={name}`), with the thing's name, percent-encoded,
  for `{name}`. It is plain bytes, as a record's is. A row homing anything
  has a plain path: steps of letters, digits and `-._~`, none empty and
  none starting with a dot. A pattern reads no `homes.json`, and a record
  never lists a kind its row gives a pattern: the pattern outranks it.
- **A mention links home.** Wherever a page names a thing that has a home,
  its first mention in each row, item, paragraph and heading is a link to
  that home, its address taken from `homes.json` or made by its kind's
  pattern, never built by hand.
  The link is the kit's link, with the name in its own face (a cell's
  address and a machine's name in Monaspace). On its own home, the thing's
  name needn't link: the view that is its home says so with `data-home`,
  its address as `homes.json` gives it. A name drawn on a walked map is the map's mark,
  reached by its keys, and the drawer it opens is where the links are. A
  link out (GitHub, eero's app) is a second act beside the home's link,
  never instead of it.
- **Each home drills one level down.** A home lists what the thing holds,
  each a mention linked to its own home:
  - a machine, its cells, its networks and what it's using;
  - a network, its cells;
  - a repository, its checkouts and trees, and each tree the cells at work
    in it;
  - a device, the machine it is, when it is one.

  A thing that is two kinds (a laptop is a machine and, on the home
  network, a device) links once from each home to the other. Up is the
  trail at the top, every crumb a link. `homes.js`, beside this file, is
  the probe that finds each mention left unlinked and each twin left
  apart (`checks.md`).

## The Kit

The theme module writes the theme for the web on every machine, at
`~/.local/share/jitsusama/web/`. A document, or a site deployed
elsewhere, must stand alone: copy the folder whole into it (as `kit/`),
and load it before the site's own CSS:

```html
<link rel="stylesheet" href="kit/theme.css">
<link rel="stylesheet" href="kit/base.css">
<link rel="stylesheet" href="site.css">
<link rel="preload" href="kit/fonts/fraunces.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="kit/fonts/fraunces-italic.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="kit/fonts/monaspace.woff2" as="font" type="font/woff2" crossorigin>
```

A live site built by `localSite` copies nothing: the sites' server holds
one copy for every live site at `/local-sites/web/`, so the browser
fetches the fonts once for them all. Its page, served one folder deep,
loads the same lines from there, preloads included, and the build refuses
a page still loading its own `kit/`:

```html
<link rel="stylesheet" href="../local-sites/web/theme.css">
<link rel="stylesheet" href="../local-sites/web/base.css">
<link rel="stylesheet" href="../local-sites/kit/switch.css">
<link rel="stylesheet" href="../local-sites/kit/head.css">
<link rel="stylesheet" href="site.css">
<link rel="preload" href="../local-sites/web/fonts/fraunces.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="../local-sites/web/fonts/fraunces-italic.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="../local-sites/web/fonts/monaspace.woff2" as="font" type="font/woff2" crossorigin>
```

The server holds two folders there that sound alike. `/local-sites/web/`
is the theme's web kit: `theme.css`, `base.css` and the fonts.
`/local-sites/kit/` is the sites' own scripts and switch: `feed.js`,
`keys.js`, `document.js`, the rest built from the kit's TypeScript, and
`switch.css` and `head.css`. Neither holds the other's files.

**The head every live page opens with** is the kit's, so the pages read
as one family; before it each drew its own, five eyebrows in five faces.
Its parts come in this order, each said once:

```html
<header class="page-head">
  <p class="eyebrow">your sites<span class="sep">·</span>on this machine</p>
  <h1>Every Site, <em>Here</em></h1>
  <p class="lede">Seven sites, each live, and two asking something of you.</p>
  <p class="standing"><span class="glyph" aria-hidden="true"></span>
    <span id="status" role="status">Reaching its daemon.</span></p>
  <div class="quiet">
    <p class="keys"><kbd data-tap>j</kbd> and <kbd data-tap>k</kbd> step from
      site to site, and <kbd>?</kbd> lists every key</p>
    <!-- the three-way switch, centred under the key line -->
  </div>
</header>
```

- **The eyebrow** is two short phrases, what this is · whose or where,
  never a sentence or three clauses.
- **The title** is Title Case, Joel's word (`rulings.md`), with one italic
  turn in the tint at most.
- **The lede** says what stands now in his terms: never a session's
  address, a sha or a command's name.
- **The standing line** is written through `status.ts`: `show` for the
  feed's state and any words the page adds after it, or `tell` for words
  the page builds itself. Each writes only when the words change, so a
  screen reader hears a change and never a repeat. Only it is a live
  region; a lede rewritten as the feed moves is not one.
- **The quiet row** holds the key line and the switch under it, as Joel's
  Crossing has them. On a phone the line's words wait for the first key
  pressed outside a field, which `listen` marks; a page answering its keys
  its own way calls `watchForKeys`. Keycaps that act when tapped keep the
  line showing: mark them `data-tap` in the markup, so the line is there at
  first paint, and make them act with `pressable`. A `?` or a help button
  alone doesn't keep it.

`head.css` reads each part only inside `.page-head`, so linking it changes
nothing until a page moves its head into one; check that move with
`shots-changed`. The `heads-are-the-kits` check reads each page's markup
against this head once the page leaves its waiting list in
`modules/local-sites/checks.nix`, and says what to change.

An interface (`data-kind="interface"` on the root) starts the same parts on
its gutter's edge rather than the centre.

**A long list folds** with `fold.ts`: at most seven rows, ranked by the
page, then a button saying how many more rows opening shows ("And 13 more
trees"), never more than it opens. A list no longer than the fold has
no button, even opened by the page's key. The page keeps the rows that
must show wherever they rank, such as the cursor's own, and owns whether
the fold is open. What Joel never acts on one by one, a repository's
snapshots, is counted, never listed: `fold.ts` gives that count apart as
`unlisted`, and the page says it in its own sentence.

**An empty place says what will fill it**, as a `.empty` sentence where
the rows would be ("Murmurs land here as sessions talk to each other"),
and is no listbox or grid until it holds a row: an empty listbox is read
out as a broken control.

Preload the faces the first screen draws: the heading's italic turn and a
label put all three on most of them.

- `theme.css` is written from the theme, so it's never edited: every colour
  (`--background`, `--text`, `--muted`, `--strong`, `--accent`, the glyph
  hues `--action`, `--program`, `--machine`, `--person`, `--voice`, the
  states) for both looks, dark and light; the faces (`--serif`, `--mono`,
  `--mono-features`) with stand-ins metric-matched so nothing moves when the
  fonts arrive; the fluid type steps (`--step-small` to `--step-hero`); each
  level's settings (`--title-weight`, `--words-soft`, `--label-tracking`);
  the space ladder's rungs (`--tie`, `--mark`, `--apart`, `--items`,
  `--groups`, `--inset`, `--frames`, a fluid `--screen`); the radius and
  border; and the motion (`--ease`, `--leave`, `--arrive`, `--large`).
- `base.css` sets the type as a typesetter would, one focus ring, one
  selection, links, code, tables, disclosures, fields, and what the reader's
  system asks for. It's a sensible floor, not the design: a site overrides
  it wherever its world says so (its own scale, its own masthead, its own
  rows), and keeps its focus ring, skip link and reduced-motion rules.
- `fonts/` holds Fraunces upright and italic and the theme's Monaspace, cut to
  what a page in English uses: about 125, 150 and 60 KB.

A site names properties, never a value. Where it needs one the kit lacks,
derive it from the colours (`color-mix()` of two) and say why in a
comment; if every site would need it, it belongs in the kit, in core.nix's
`modules/theme/web/`.

**A site's own tint.** Each site has its own feel, as every one of his
favourites did (`language.md`, Colour). Two properties turn the kit's
palette, and a site may go further and name its own pigments, each with
one job, as Slack Water does (`examples/tracker.md`):

- `--turn` turns the grounds, the text and the accent round the hue wheel, in
  degrees, keeping their lightness, so contrast holds: `:root { --turn: 85 }`
  gives a harbour blue in place of the forest green ("with a blue tint instead
  of green", 2026-11-03).
- `--tint` is the hue the site marks its emphasis in: the italic turn of a
  heading, eyebrows, drop caps, ornaments, a link's underline on hover. It's
  gold (`--action`) unless the site takes another glyph hue as its own:
  `:root { --tint: var(--program) }` for sea glass. Gold then keeps its own
  meaning, something running or partly so.

**Dark and light.** Both looks are in `theme.css`, chosen by the reader's
system, dark when it says nothing. Give every site the three-way switch he
asked for, "a 3way with icons toggle, auto/dark/light, and make sure it looks
spiffy" (2026-10-02): Light, Match System and Dark, small and set into
the page's composition (the Crossing's sits centred under its key line), applied
before the first paint so nothing flashes (`craft.md` has it). A site
that is dark only sets `data-theme="dark"` on `<html>`.

**The keys and the switch, as omni-ref had them**: "i like how [it] updated
itself with different light styles and the ? help, i think that should be
good general guidance for any page we create" (2026-11-08). He liked what
they do, not a place for them: set them into the page's own composition,
never as a strip of chrome across the top (`examples/what-went-wrong.md`).
Every page, either kind, has:

- **One quiet line saying how to move**: "j and k step from part to part,
  and ? lists every key", small and muted, the keys drawn as keycaps, under
  the contents on a page to read, in the head's quiet row on an interface.
- **? opens a Keys list** and closes it again: j next part, k previous, g
  first, G last, ? this list, Escape closing it too. An interface's list adds
  its own keys after these.
- **The three-way switch, under that line**, Match System chosen until the
  reader picks, and remembered across visits.
