# What more than one of the theme's modules reads: the theme file, a value
# it may leave out, the shape of a number it gives, and the colour roles,
# which the type's levels name as well as colour.nix.
{ lib, cfg }:
let
  theme = builtins.fromTOML (builtins.readFile cfg.file);

in
{
  inherit theme;

  # A table the theme file may leave out, and a value in it, falling back to
  # this module's default.
  given =
    table: key: default:
    (theme.${table} or { }).${key} or default;

  number = lib.types.strMatching "[0-9]+(\\.[0-9]+)?";

  # The colour roles, by what each means.
  colours = {
    background = "behind everything: windows, the terminal, menus";
    sunken = "below the background: a sidebar, an inactive tab, a list";
    deepest = "the very back: the overview's backdrop, a tab bar";
    raised = "above the background: a hovered row, an indicator";
    selection = "behind what's selected";
    text = "what Joel reads and acts on";
    strong = "text that stands out: a heading, a selected row, the cursor";
    muted = "text that gives context: labels, units, hints";
    rule = "separators and marks that aren't read as text";
    edge = "the outline of what isn't focused: a window, a field at rest";
    shadow = "the shadow under a floating surface, drawn faint";
    accent = "where Joel's attention is: one focal point per screen state";
    on_accent = "text drawn on the accent or on any of the state colours";
    alert = "something that needs Joel: a failure, an urgent window";
    warning = "something that will need him soon";
    success = "something that went well";
    # The hues glyphs wear, never text: each is one kind of thing, wherever
    # it's drawn (docs/design.md, Colour).
    action = "a glyph for something to do: a command, a shortcut";
    program = "a glyph for a program: an application, where a notice came from";
    machine = "a glyph for the machine itself: power, security, hardware";
    person = "a glyph for a person: a message, a mention, a call";
    voice = "a glyph for the desktop speaking for itself: an empty list, a greeting";
  };
}
