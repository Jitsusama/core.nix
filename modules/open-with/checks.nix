# The checks of open-with, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the portal's chooser of what to open something
  # with, its tests included.
  open-with-passes-clippy = passesClippy "open-with-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # xdg-open opens a kind by the kinds it's a case of, in its declared app,
  # and a program that runs in a terminal in the terminal named for it:
  # markdown in the editor declared for text, in the terminal, and a
  # picture in its viewer. The apps and the terminal are stand-ins that
  # say what they were given; gio and xdg-terminal-exec are the real ones
  # (tests/xdg-open-opens-each-kind.sh).
  xdg-open-opens-each-kind-in-its-app =
    pkgs.runCommandLocal "xdg-open-opens-each-kind-in-its-app"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./xdg-open.nix { })
          pkgs.xdg-terminal-exec
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/xdg-open-opens-each-kind.sh} ${pkgs.shared-mime-info}
        touch $out
      '';
}
