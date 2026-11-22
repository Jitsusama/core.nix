# The Theme on the Web

What the theme module gives a website, on every machine:

- `web.nix` writes the kit a site copies whole, to `~/.local/share/jitsusama/web/`:
  `theme.css`, generated from the theme; `base.css`, beside it, written by hand
  and sent without its comments, which stay here; and the two faces, cut small
  by `fonts.py`.
- `skill/`: the web-design skill, installed for every agent beside the
  aesthetic skill. `SKILL.md` is short: how to think about a site and the
  process for designing one, drawn from the four sites Joel is proud of,
  with an index of the files beside it, each loaded when a step needs it:
  `examples/` (each loved site taken apart, and what went wrong on 15
  November), `language.md` (the design language), `rulings.md` (his
  verdicts), `interface.md`, `sites.md` (the local home and the kit) and
  `gates.md` (what must pass, at verification);
  `writing.md`, `craft.md` and `checks.md` are what it refers to, and
  `gate.js` is the probe `checks.md` runs in a page first, before it's
  published, naming each part it lacks and each fault in its words, and
  `layout.js` the one after it, naming each row and fold that doesn't hold;
  `map.js` walks a map or graph by its keys, naming each arrow that breaks
  the one rule and each drawn name over a line or another name; and
  `homes.js` reads every site's `homes.json`, naming each mention of a thing
  that doesn't link to its home; and `looks.js`, run on a live page before
  a deploy and after it, names each look the deploy lost.
- `homes-test.py` holds `homes.js` to its cases in headless Chromium, as the
  `homes-probe-reads-as-the-reader` check runs it, and `looks-test.py`
  holds `looks.js` to its, as `looks-probe-finds-what-a-deploy-lost` does;
  both read their pages in the one Chromium `probe_browser.py` opens.

A rule every site would need goes in `base.css`; a value every site would
need goes in `theme.css` through `web.nix`; a verdict of his on a site goes in
the skill, with his words and the date.
