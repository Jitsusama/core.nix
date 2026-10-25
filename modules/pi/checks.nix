# The checks of pi, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  pkgs,
  workstation,
  ...
}:
{
  # pi's theme is the machine's: its text, its code and its thinking levels
  # are the theme's colours, and every colour it names is one pi reads.
  pi-wears-the-theme =
    let
      home = account workstation;
      theme = builtins.fromJSON home.home.file.".pi/agent/themes/jitsusama.json".text;
      colours = lib.attrValues theme.colors ++ lib.attrValues theme.export;
    in
    holds "pi-wears-the-theme" "pi's theme isn't the machine's, or names a colour pi can't read." (
      theme.colors.text == home.jitsusama.theme.roles.text
      && theme.colors.syntaxString == home.jitsusama.theme.syntax.string.colour
      && theme.colors.thinkingMax == home.jitsusama.theme.roles.alert
      && lib.all (colour: builtins.match "#[0-9a-fA-F]{6}" colour != null) colours
    );

  # pi's status line tells a folder and git as the prompt does: by the
  # prompt's root markers, the prompt's git-engine, and the theme's marks,
  # each by the name the harness reads it under.
  pi-tells-where-as-the-prompt-does =
    let
      home = account workstation;
      where = builtins.fromJSON (
        builtins.unsafeDiscardStringContext
          home.xdg.configFile."pi/agentic-harness.pi/status-line/where.json".text
      );
      inherit (home.jitsusama.theme) glyphs;
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
    in
    holds "pi-tells-where-as-the-prompt-does"
      "pi's status line tells the folder or git by something other than the prompt's roots, engine and marks."
      (
        where.roots == home.jitsusama.prompt.roots
        && where.engine == builtins.unsafeDiscardStringContext engine
        &&
          where.marks == {
            branch = glyphs.branch.glyph;
            unsaved = glyphs.on.glyph;
            unpushed = glyphs.unpushed.glyph;
            more = glyphs.more.glyph;
            complete = glyphs.complete.glyph;
            active = glyphs.active.glyph;
            pending = glyphs.pending.glyph;
            thinking = glyphs.thinking.glyph;
          }
      );

  # pi's settings file gets the settings declared for it at every switch,
  # core's and a machine's own, and keeps every key pi wrote there; a file
  # that isn't JSON is left alone and the switch says so; and a file already
  # holding them isn't written again.
  pi-keeps-its-own-settings =
    let
      home = account workstation;
      program = lib.getExe (
        import ./settings.nix {
          inherit pkgs;
          inherit (home.jitsusama.pi) settings;
        }
      );
      # As a machine adds its own, such as the model it starts with.
      machines = lib.getExe (
        import ./settings.nix {
          inherit pkgs;
          settings = home.jitsusama.pi.settings // {
            defaultModel = "a-model";
          };
        }
      );
    in
    assert lib.assertMsg (lib.hasInfix (builtins.unsafeDiscardStringContext program) (
      builtins.unsafeDiscardStringContext home.home.activation.piTakesCoresSettings.data
    )) "pi's activation doesn't run the settings program the check tests.";
    pkgs.runCommandLocal "pi-keeps-its-own-settings" { nativeBuildInputs = [ pkgs.jq ]; } ''
      file=$TMPDIR/agent/settings.json
      mkdir -p $TMPDIR/agent
      # As pi writes it, with a reserve of its own that core's replaces.
      echo '{"theme":"jitsusama","lastChangelogVersion":"1.3.0","compaction":{"enabled":true,"reserveTokens":16384}}' >$file
      ${program} $file
      jq -e '.theme == "jitsusama" and .lastChangelogVersion == "1.3.0"' $file
      jq -e '.compaction == {"enabled": true, "reserveTokens": 64000}' $file

      touch -d 2000-01-01 $file
      ${program} $file
      [ "$(stat -c %Y $file)" = "$(date -d 2000-01-01 +%s)" ]

      echo '{"theme":' >$file
      if ${program} $file; then exit 1; fi
      [ "$(cat $file)" = '{"theme":' ]

      rm $file
      ${program} $file
      jq -e '.compaction.reserveTokens == 64000' $file

      echo '{"theme":"jitsusama","defaultModel":"picked-in-pi"}' >$file
      ${machines} $file
      jq -e '.theme == "jitsusama" and .defaultModel == "a-model"' $file
      jq -e '.compaction.reserveTokens == 64000' $file
      touch $out
    '';
}
