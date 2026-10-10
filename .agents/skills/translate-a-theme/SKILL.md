---
name: translate-a-theme
description: >
  Translate a colour theme from another format (Omarchy's colors.toml, base16
  or base24, kitty, Ghostty, iTerm2, VS Code, a Neovim colour scheme, or a
  screenshot) into core.nix's own theme format, modules/theme/<name>.toml.
  Use when asked to add, port, import or try a theme.
---

# Translate a Theme

core.nix reads one theme format, its own, and has no importer: a theme from
anywhere else is translated once, by hand, by following this skill, and the
result is committed. [docs/design.md][design] says what a theme is for; its
Colour and Themes sections are the contract this file is written to. Read
them first, and read `modules/theme/osaka-jade.toml` as the worked example.

## The Shape

A theme is one file, `modules/theme/<name>.toml`, named in kebab case, and
listed in `themes` in `modules/theme/home.nix`. It holds:

- `name`, `appearance` (`dark` or `light`) and `icons`, the Yaru icon theme
  nearest its accent (`Yaru-sage`, `Yaru-blue`, and so on).
- `shades`: eight pigments from the background to the brightest text.
- `[palette]`: every colour the theme uses, once, as `#rrggbb` or
  `#rrggbbaa`.
- `[roles]`: what each colour means. Every role `modules/theme/home.nix`
  declares has to be given, each naming a pigment.
- `[terminal]`: the terminal's own six colours and its sixteen ANSI ones,
  each naming a pigment.

Anything else the format grows (type, space, motion, glyphs, wallpaper) falls
back to the theme module's defaults, so leave it out unless the source theme
says something about it.

## Steps

1. **Find the source's colours.** Where the usual formats keep them:
   - Omarchy `colors.toml`: flat keys. `accent`, `selection`, `muted`, four
     backgrounds (`darker_background` to `lighter_background`), four
     foregrounds, and the sixteen terminal colours by name. Its `blue` is
     often its `accent`.
   - base16: `base00` to `base07` are the shades, dark to light; `base08` to
     `base0F` are red, orange, yellow, green, cyan, blue, magenta and brown.
     base24 adds `base10` and `base11` (darker backgrounds) and `base12` to
     `base17` (the bright colours).
   - kitty and Ghostty: `foreground`, `background`, `cursor`,
     `selection_*`, and `color0` to `color15` (Ghostty's `palette = N=#...`).
   - iTerm2 `.itermcolors`: `Ansi 0 Color` to `Ansi 15 Color`, plus
     `Foreground`, `Background`, `Cursor` and `Selection Color`, each as
     float components to turn into hex.
   - VS Code: `colors` holds the interface (`editor.background`,
     `focusBorder`, `button.background`, `list.activeSelectionBackground`,
     `editorError.foreground`), and `terminal.ansi*` the terminal's.
   - A Neovim scheme: the `Normal`, `NormalFloat`, `Visual`, `Comment`,
     `CursorLine`, `DiagnosticError` and `DiagnosticWarn` groups.
   - A screenshot: sample the pixels, and say in the file's header that it
     was sampled rather than copied.

2. **Name the pigments by what they look like:** `jade`, `parchment`,
   `night`, never `accent` or `background`. The role says what a colour is
   for; the pigment's name only has to tell two greens apart. Keep every
   colour the source has, even unused ones, and note beside each the name
   the source gives it, so the translation can be checked against it.

3. **Choose the roles.** Most map directly; some are judgement:
   - `background`, `sunken`, `deepest`, `raised`: the source's main
     background and the ones just below and above it. A source with one
     background gets the others by darkening and lightening it a step in
     OKLCH lightness, as new pigments.
   - `text` and `strong`: the main foreground and the brightest one.
   - `muted` is text, so it has to read: 4.5:1 on the background, sunken and
     deepest. A source's comment or muted colour is usually fainter than
     that; keep it as `rule`, and make `muted` a new pigment of its hue made
     just light enough to pass, as Osaka Jade's `sage` is.
   - `accent` is the one colour that means "here", so it's the source's
     focus or cursor colour, not merely its most saturated one.
   - `on_accent` is text on the accent and on the state colours, usually
     the darkest shade.
   - `alert`, `warning`, `success`: the source's error, warning and success
     colours, or its red, yellow and green.

   Mark every role that wasn't a direct copy with a comment saying what was
   decided and why. Those are what Joel checks by eye.

4. **Fill the terminal.** The source's own sixteen colours when it has them.
   Every one has to read on the terminal's background except the ones at
   the background's own end: `black` on a dark theme, `white` and
   `bright_white` on a light one. `bright_black` is text too (shell
   suggestions, comments), so it has to read like `muted`. A light theme's
   brighter inks are usually too faint on paper, so take the darker inks of
   the same family and say so; `flexoki-light.toml` is the worked example.

5. **Add it to `themes`** in `modules/theme/home.nix`.

## Proving It

- **Legibility.** Build any check with the theme picked
  (`jitsusama.theme.name = "<name>"` on a bare machine, as
  `a-faint-theme-is-refused` in `checks.nix` does). Every text pair under
  4.5:1 is listed by name with its ratio. Fix each by adding a pigment, never
  by loosening the check.
- **Nothing hard-coded.** A module that names a colour of its own isn't a
  theme problem; fix the module, so the theme reaches it.
- **By eye.** Switch a machine to the theme and screenshot the desktop, a
  terminal with a listing, a diff and Neovim open, beside the source's own
  screenshot. The translation is done when the two read as the same theme,
  and every judgement comment has been looked at.

[design]: ../../../docs/design.md
