# The Live Sites' Kit

What every live local site shares, in TypeScript for the browser, built
and tested by [`site.nix`][site] and served at `/local-sites/kit/`, where
a site's compiled imports land.

| File          | What it holds                                                 |
| ------------- | ------------------------------------------------------------- |
| `ask.ts`      | a look read for its ask row, and the answers it may send      |
| `asks.ts`     | the looks waiting on Joel, drawn as ask rows and answered there |
| `answer.ts`   | an answer pressed on a page, posted, and what came of it said |
| `cell.ts`     | a cell's link to the launcher, and the cell a link names      |
| `document.ts` | a document's parts, its keys and its switch, wired on load    |
| `early.js`    | the kept look, `js` and a page's kept room, before it's drawn |
| `feed.ts`     | a watch of a daemon's feed: connecting, live, stale or broken |
| `fold.ts`     | a long list folded to seven rows, how many more, what's unlisted |
| `home.ts`     | the home's cursor over the sites, its keys and its switch     |
| `keyed.ts`    | a list drawn by record key, so an update touches only its rows |
| `known.ts`    | how a keyed list knows a focused control again in its new drawing |
| `keys.ts`     | the keys a page answers, and its list of them on `?` and F1   |
| `records.ts`  | a daemon's records, held as its watch tells them              |
| `status.ts`   | the line saying how live a page is and since when             |
| `steps.ts`    | stepping a cursor, gg's two presses and keys pressed by tap   |
| `theme.ts`    | the theme switch, on real radios, and the look it keeps       |
| `switch.css`  | the switch's styles                                           |
| `head.css`    | the head every page opens with, and an empty place's sentence |
| `ask.css`     | the ask row's styles                                          |
| `tests/`      | each of them tried against a stand-in socket and clock        |

The `tests/` folder holds a test for each script but `theme.ts`,
`home.ts` and `early.js`, which have none yet (the pages' live tests open
them in a browser), and `known.ts`, which keyed's focus cases
(`keyed-focus.test.ts`, and `keyed-form.test.ts` for forms) try through the
list, drawn by `focused.ts` in `rows.ts`'s list; and the stand-ins they share,
`stand-in.ts` for a feed's socket and `hands.ts` for a clock moved by hand. Node runs them
as the kit builds.

The kit is typechecked with `noPropertyAccessFromIndexSignature`, so a field
a record may lack is read as `record["field"]` and only a declared one as
`record.field`. A site extending its `tsconfig.json` takes the flag too.

[site]: ../site.nix
