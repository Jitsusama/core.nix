# The checks of open-with, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
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
  # say what they were given; gio and xdg-terminal-exec are the real ones.
  xdg-open-opens-each-kind-in-its-app =
    pkgs.runCommandLocal "xdg-open-opens-each-kind-in-its-app"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./xdg-open.nix { })
          pkgs.xdg-terminal-exec
        ];
      }
      ''
        export HOME=$TMPDIR/home XDG_CONFIG_HOME=$TMPDIR/config XDG_CURRENT_DESKTOP=niri
        export XDG_DATA_DIRS=$TMPDIR/share:${pkgs.shared-mime-info}/share
        mkdir -p $HOME $XDG_CONFIG_HOME $TMPDIR/share/applications $TMPDIR/bin
        export PATH=$TMPDIR/bin:$PATH
        for app in editor viewer terminal; do
          printf '#!${pkgs.runtimeShell}\necho "%s $*" >> %s\n' $app $TMPDIR/opened > $TMPDIR/bin/$app
          chmod +x $TMPDIR/bin/$app
        done
        entry() {
          printf '[Desktop Entry]\nType=Application\nName=%s\n%s\n' "$1" "$2" \
            > $TMPDIR/share/applications/$1.desktop
        }
        entry editor $'Exec=editor %F\nTerminal=true\nMimeType=text/plain;'
        entry viewer $'Exec=viewer %f\nMimeType=image/png;'
        entry terminal $'Exec=terminal\nCategories=System;TerminalEmulator;\nX-TerminalArgExec=--'
        printf '[Default Applications]\ntext/plain=editor.desktop\nimage/png=viewer.desktop\n' \
          > $XDG_CONFIG_HOME/niri-mimeapps.list
        echo terminal.desktop > $XDG_CONFIG_HOME/xdg-terminals.list

        cd $TMPDIR
        echo '# Notes' > notes.md
        base64 -d <<< iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg== > shot.png
        xdg-open notes.md
        xdg-open shot.png
        for _ in $(seq 50); do
          [ "$(wc -l < opened 2>/dev/null || echo 0)" -ge 2 ] && break
          sleep 0.1
        done
        cat opened
        grep -qx "terminal -- editor $TMPDIR/notes.md" opened
        grep -qx "viewer $TMPDIR/shot.png" opened
        touch $out
      '';
}
