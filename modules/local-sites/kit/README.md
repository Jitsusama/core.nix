# The Live Sites' Kit

What every live local site shares, in TypeScript for the browser, built
and tested by [`site.nix`][site] and served at `/local-sites/kit/`, where
a site's compiled imports land.

| File          | What it holds                                                 |
| ------------- | ------------------------------------------------------------- |
| `feed.ts`     | a watch of a daemon's feed: connecting, live, stale or broken |
| `records.ts`  | a daemon's records, held as its watch tells them              |
| `status.ts`   | the line saying how live a page is and since when             |
| `theme.ts`    | the theme switch, on real radios, and the look it keeps       |
| `switch.css`  | the switch's styles                                           |
| `tests/`      | each of them tried against a stand-in socket and clock        |

[site]: ../site.nix
