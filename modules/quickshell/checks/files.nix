# The checks of the launcher's Files, and the file chooser it answers for
# programs, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # A file in the launcher's chooser of files says its size and when it
  # changed as VKCF3Z set them, and its plate its kind, preview and facts,
  # one kept elsewhere, as in Google Drive, alike from what its place
  # listed, a file nothing has read is its ending's kind and a picture
  # previewed as it is, and a file in the Nix store goes by its own name
  # past its hash, as tests/file-facts.qml holds FileFacts.js to it in Qt's
  # own engine.
  a-file-says-its-size-and-when-it-changed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-file-says-its-size-and-when-it-changed" { } ''
      cp ${../../../tests/file-facts.qml} file-facts.qml
      ${copied [ "FileFacts.js" ]}
      cp ${../launcher/words.json} words.json
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml file-facts.qml
      touch $out
    '';

  # The launcher chooses the files a program opens or saves, as open-with
  # asks it (tests/file-chooser.qml): in folders made here, with the files
  # opened lately listed by a stand-in for recent-files, the desktop's
  # kinds of file by a few of shared-mime-info's lines, a place kept
  # elsewhere by a stand-in for Google's Drive, file-preview by one that
  # notes which process each thing was asked of, and niri by one whose
  # window with the keys is a text editor's, the program that asks.
  the-launcher-chooses-files-as-asked =
    let
      niri = singleton "Niri.qml" ''
        property var windows: ({ "7": { id: 7, app_id: "org.gnome.TextEditor" } })
        property var current: 7
      '';
      # A stand-in for file-preview, whether kept for the ask (serve) or
      # started for each thing asked: it says there's nothing to show and
      # notes each thing asked beside the process it was asked of.
      previewer = pkgs.writeShellScript "file-preview" ''
        answer() {
          printf '%s %s\n' "$$" "$1" >> "$HOME/previewed"
          case $1 in
            show\ *) printf '{"error": "a stand-in"}\n' ;;
            *) printf '{}\n' ;;
          esac
        }
        if [ "$1" = serve ]; then
          while IFS= read -r line; do answer "$line"; done
        else
          answer "$*"
        fi
      '';
    in
    pkgs.runCommandLocal "the-launcher-chooses-files-as-asked" { } ''
      mkdir -p launcher kinds
      cp ${niri} Niri.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Files.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/Recents.js"
        "launcher/words.json"
      ]}
      cp ${../../open-with/FileChooser.qml} launcher/FileChooser.qml
      cp ${../../../tests/file-chooser.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir $XDG_RUNTIME_DIR/open-with
      export FONTCONFIG_FILE=${noFonts}
      # A folder of notes, two hidden things in it, and a file opened lately
      # elsewhere beside one of its own; notes.txt is the largest, then b.png.
      # Two folders deep elsewhere go from one to the other on ]b. The list
      # opened lately is as recent-files prints it, its time last, empty
      # where it's not known.
      mkdir -p $HOME/notes/sub $HOME/notes/.secret $HOME/elsewhere/deep/one $HOME/elsewhere/deep/two
      touch $HOME/notes/a.md $HOME/notes/.hidden.txt
      printf 'twelve bytes' > $HOME/notes/notes.txt
      printf 'png' > $HOME/notes/b.png
      touch $HOME/elsewhere/old.md
      printf 'Notes\t%s\t1700000000\nNotes\t%s\t\n' "$HOME/elsewhere/old.md" "$HOME/notes/a.md" \
        > $HOME/recent
      export RECENT_LIST=${pkgs.writeShellScript "recent-list" ''cat "$HOME/recent"''}
      printf '50:text/markdown:*.md\n50:image/png:*.png\n50:text/plain:*.txt\n' > kinds/globs2
      printf 'text/markdown text/plain\n' > kinds/subclasses
      export KINDS=$PWD/kinds PREVIEWER=${previewer}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      asked_of() { grep -m1 " show $1$" $HOME/previewed | cut -d' ' -f1; }
      a=$(asked_of $HOME/notes/a.md)
      if [ -z "$a" ] || [ "$(asked_of $HOME/elsewhere/old.md)" != "$a" ]; then
        echo "the files selected in one ask weren't asked of one file-preview:"
        cat $HOME/previewed
        exit 1
      fi
      [ -d $HOME/staged/plans ] || { echo "saving into a place kept elsewhere made no folder"; exit 1; }
      from=$HOME/staged/plans/.new.md.from
      printf org.gnome.TextEditor | cmp -s - $from || { echo "a save staged elsewhere doesn't say beside it which program asked: $(cat $from 2>&1)"; exit 1; }
      touch $out
    '';
}
