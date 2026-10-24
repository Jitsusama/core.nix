# shell-preview, for the dev shell: the desktop's shell from the working copy, in a niri
# window of its own. shell-preview.py says how it's used, and its tests are
# shell-preview-test.py. ruff and mypy read both through `nix fmt`, at their strictest, so the
# writers' own flake8 pass is left off.
{
  dbus,
  lib,
  writers,
  wtype,
}:
{
  tool = writers.writePython3Bin "shell-preview" { doCheck = false; } (
    builtins.replaceStrings
      [
        "@dbus_run_session@"
        "@wtype@"
      ]
      [
        (lib.getExe' dbus "dbus-run-session")
        (lib.getExe wtype)
      ]
      (builtins.readFile ./shell-preview.py)
  );
  tests = writers.writePython3 "shell-preview-test" { doCheck = false; } (
    builtins.readFile ./shell-preview-test.py
  );
}
