# The Theme on the Web

What the theme module gives a website, on every machine:

- `web.nix` writes the kit a site copies whole, to `~/.local/share/jitsusama/web/`:
  `theme.css`, generated from the theme; `base.css`, beside it, written by hand;
  and the two faces, cut small by `fonts.py`.
- `skill/`: the web-design skill, installed for every agent beside the
  aesthetic skill. `SKILL.md` says how a site is built to Joel's taste and why;
  `writing.md`, `craft.md` and `checks.md` are what it refers to, and
  `gate.js` is the probe `checks.md` runs in a page first, before it's
  published, naming each part it lacks and each fault in its words, and
  `layout.js` the one after it, naming each row and fold that doesn't hold.

A rule every site would need goes in `base.css`; a value every site would
need goes in `theme.css` through `web.nix`; a verdict of his on a site goes in
the skill, with his words and the date.
