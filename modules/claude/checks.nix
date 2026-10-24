# The checks of Claude Code's module, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  pkgs,
  workstation,
  ...
}:
let
  linux = (account workstation).programs.claude-code;
  mac = (account macWorkstation).programs.claude-code;
in
{
  # Both kinds of workstation show Claude Code the status line, in settings
  # it can still write.
  claude-shows-its-status-line =
    holds "claude-shows-its-status-line"
      "Claude Code isn't told of the status line, or its settings would be read-only."
      (
        lib.all
          (
            claude:
            claude.mutableSettings
            && claude.settings.statusLine.type == "command"
            && lib.hasSuffix "/bin/claude-statusline" claude.settings.statusLine.command
          )
          [
            linux
            mac
          ]
      );

  # The status line shows the folder, the model and the context's size in
  # the theme's colours, as a workstation builds it
  # (tests/claude-statusline-test.py).
  claude-status-line-says-how-it-stands =
    let
      tests = pkgs.writers.writePython3 "claude-statusline-test" { doCheck = false; } (
        builtins.readFile ../../tests/claude-statusline-test.py
      );
    in
    pkgs.runCommandLocal "claude-status-line-says-how-it-stands" { } ''
      CLAUDE_STATUSLINE=${linux.settings.statusLine.command} ${tests}
      touch $out
    '';
}
