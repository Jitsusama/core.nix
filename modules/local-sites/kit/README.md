# The Live Sites' Kit

What every live local site shares, in TypeScript for the browser, built
and tested by [`site.nix`][site] and served at `/local-sites/kit/`, where
a site's compiled imports land.

| File          | What it holds                                                 |
| ------------- | ------------------------------------------------------------- |
| `document.ts` | a document's parts, its keys and its switch, wired on load    |
| `feed.ts`     | a watch of a daemon's feed: connecting, live, stale or broken |
| `home.ts`     | the home's cursor over the sites, its keys and its switch     |
| `keys.ts`     | the keys a page answers, and its list of them on `?` and F1   |
| `records.ts`  | a daemon's records, held as its watch tells them              |
| `status.ts`   | the line saying how live a page is and since when             |
| `steps.ts`    | stepping a cursor, gg's two presses and keys pressed by tap   |
| `theme.ts`    | the theme switch, on real radios, and the look it keeps       |
| `switch.css`  | the switch's styles                                           |
| `tests/`      | each of them tried against a stand-in socket and clock        |

The `tests/` folder holds a test for each script but `theme.ts` and
`home.ts`, which have none yet, and the stand-ins they share, `stand-in.ts`
for a feed's socket and `hands.ts` for a clock moved by hand. Node runs them
as the kit builds.

The kit is typechecked with `noPropertyAccessFromIndexSignature`, so a field
a record may lack is read as `record["field"]` and only a declared one as
`record.field`. A site extending its `tsconfig.json` takes the flag too.

[site]: ../site.nix
