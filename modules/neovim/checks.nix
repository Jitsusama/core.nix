# The checks of neovim, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
let
  # Puts an account's Neovim where Neovim looks for it, under $PWD: every
  # file the account has in ~/.config/nvim, and the plugins.
  neovimAt =
    home:
    let
      files = home.xdg.configFile;
      names = lib.filter (lib.hasPrefix "nvim/") (lib.attrNames files);
    in
    ''
      export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
      export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache
      mkdir -p data/nvim/site/pack
      ln -s ${home.xdg.dataFile."nvim/site/pack/hm".source} data/nvim/site/pack/hm
      ${lib.concatMapStrings (name: ''
        mkdir -p config/${dirOf name}
        cp ${files.${name}.source} config/${name}
      '') names}
    '';
  # Runs Neovim headless with a probe and fails unless it answers as
  # expected (probe.sh).
  probed = "${lib.getExe pkgs.bash} ${./probe.sh}";
  # What runs a probe once Neovim has entered (probes/at-first-screen.lua).
  atFirstScreen =
    probe: "--cmd \"let g:probe = '${probe}'\" --cmd 'luafile ${./probes/at-first-screen.lua}'";
in
onLinux {
  # The launcher lists Neovim as Neovim: the account's entry for it says so,
  # where nixpkgs' wrapper calls it "Neovim wrapper", and still runs nvim.
  neovim-is-named-neovim =
    let
      home = account workstation;
      entry = "${home.home.path}/share/applications/nvim.desktop";
    in
    pkgs.runCommandLocal "neovim-is-named-neovim" { } ''
      grep -qx 'Name=Neovim' ${entry} || { echo "the entry is named $(grep '^Name=' ${entry})"; exit 1; }
      grep -qx 'Exec=nvim %F' ${entry} || { echo "the entry runs $(grep '^Exec=' ${entry})"; exit 1; }
      touch $out
    '';

  # Neovim starts with the configuration an account gets, in the theme's own
  # colour scheme drawn on the terminal's own background, with Tree-sitter
  # drawing a file in a language it has a parser for and its text objects
  # mapped, and nothing it loads complains.
  neovim-starts-in-the-theme =
    let
      home = account workstation;
      report = lib.concatStringsSep " .. ' ' .. " [
        "vim.g.colors_name"
        "tostring(vim.api.nvim_get_hl(0, { name = 'Normal' }).bg)"
        "tostring(vim.treesitter.highlighter.active[vim.api.nvim_get_current_buf()] ~= nil)"
        "(vim.fn.mapcheck('af', 'o') ~= '' and 'objects' or 'none')"
      ];
    in
    pkgs.runCommandLocal "neovim-starts-in-the-theme"
      {
        # The programs the account gets, git and gh among them, which
        # plugins call as they start.
        nativeBuildInputs = [ home.home.path ];
      }
      ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        ${probed} 'jitsusama nil true objects' probe.lua -c "lua io.stdout:write(${report})" -c 'qa!'
        touch $out
      '';

  # Neovim starts with only what draws its first screen: with a file open,
  # no plugin that waits has loaded, nor vim.diagnostic, yet every command
  # a key runs is there, standing in. A command run, a module required and
  # an action asked for each load their plugin, the background ones load
  # once the first screen is drawn, and the Lua language server reads
  # only Neovim's own runtime.
  neovim-starts-with-only-what-it-draws =
    let
      home = account workstation;
      probe = ./probes/starts-with-only-what-it-draws.lua;
    in
    pkgs.runCommandLocal "neovim-starts-with-only-what-it-draws"
      { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        ${probed} 'as designed' ${atFirstScreen probe} probe.lua
        touch $out
      '';

  # Neovim's floats stand apart by their shade, with a step of it round
  # their words rather than a line, and nothing behind them shows through:
  # a problem shown in full, a language server's note, a notice, the
  # suggestions, the outline and a change's preview. A notice's mark comes
  # before its words.
  neovim-floats-by-shade =
    let
      home = account workstation;
      probe = ./probes/floats-by-shade.lua;
    in
    pkgs.runCommandLocal "neovim-floats-by-shade" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      echo 'return {}' > probe.lua
      ${probed} 'as designed' ${atFirstScreen probe} probe.lua
      touch $out
    '';

  # A markdown file's pictures are found where it names them, relative to
  # the file, so a terminal with kitty's graphics draws them under their
  # lines, and a picture other than PNG can be turned into one.
  neovim-finds-a-documents-pictures =
    let
      home = account workstation;
      probe = ./probes/finds-a-documents-pictures.lua;
    in
    pkgs.runCommandLocal "neovim-finds-a-documents-pictures" { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        mkdir plans attachments
        cp ${pkgs.nixos-icons}/share/icons/hicolor/16x16/apps/nix-snowflake.png attachments/chart.png
        printf '# A Plan\n\nThe chart:\n\n![the chart](../attachments/chart.png)\n' > plans/sheet.md
        ${probed} 'as designed' ${atFirstScreen probe} plans/sheet.md
        touch $out
      '';

  # Space in any of Neovim's groups finds among what it holds: every group
  # keys.nix gives keys has its own Space, and one whose keys are things
  # to do lists exactly its keys that work here, each by what it does in
  # strong and then its keys in text, in a column of their own, titled by
  # the group, and runs the one chosen as if pressed. A group with nothing
  # that works here says so rather than opening an empty list.
  neovim-finds-in-every-group =
    let
      home = account workstation;
      chords = map (key: key.chord) (import ./keys.nix);
      groups = lib.unique (
        lib.concatMap (
          chord:
          let
            words = lib.splitString " " chord;
          in
          lib.optionals (lib.head words == "Space" && lib.length words > 2) (
            map (length: lib.concatStringsSep " " (lib.take length words)) (lib.range 2 (lib.length words - 1))
          )
        ) chords
      );
      lacking = lib.filter (group: !(lib.elem "${group} Space" chords)) groups;
      probe = ./probes/finds-in-every-group.lua;
    in
    if lacking != [ ] then
      holds "neovim-finds-in-every-group"
        "These groups have no Space: ${lib.concatStringsSep ", " lacking}."
        false
    else
      pkgs.runCommandLocal "neovim-finds-in-every-group" { nativeBuildInputs = [ home.home.path ]; } ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        ${probed} 'as designed' ${atFirstScreen probe} probe.lua
        touch $out
      '';

  # Neovim's status line, drawn for a file in a repository with a change
  # not staged and a file untracked, with git's state from the engine: the
  # file told from its root, the branch with git's `*?` after it, Vim's
  # [+] once the buffer changes, an error counted, the mode once it isn't Normal, and
  # all of it fitting a narrow window.
  neovim-draws-its-status-line =
    let
      home = account workstation;
      probe = ./probes/draws-its-status-line.lua;
    in
    pkgs.runCommandLocal "neovim-draws-its-status-line" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      mkdir -p project/sub
      # Text, which no language server reads, so none reports its progress.
      echo 'a first line' > project/sub/notes.txt
      git -C project init -q -b main
      git -C project add -A
      git -C project -c user.name=check -c user.email=check@localhost -c commit.gpgsign=false \
        commit -q -m first
      echo 'a second line' >> project/sub/notes.txt
      echo 'not yet added' > project/new.txt
      ${probed} 'as designed' project/sub/notes.txt -c 'luafile ${probe}'
      touch $out
    '';

  # Neovim's numbers stand in one column: the line the cursor is on says
  # its own number, marked, right-aligned with the counts above and below
  # it rather than pushed left, and a wrapped line's later rows say none.
  neovim-keeps-its-numbers-in-one-column =
    let
      home = account workstation;
      probe = ./probes/keeps-its-numbers-in-one-column.lua;
    in
    pkgs.runCommandLocal "neovim-keeps-its-numbers-in-one-column"
      { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        seq 1 120 > lines.txt
        ${probed} 'as designed' lines.txt -c 'luafile ${probe}'
        touch $out
      '';

  # Neovim's spell check in a markdown file knows the names this writing
  # uses, and still finds a word spelt wrong.
  neovim-knows-the-names-it-writes =
    let
      home = account workstation;
      probe = ./probes/knows-the-names-it-writes.lua;
    in
    pkgs.runCommandLocal "neovim-knows-the-names-it-writes" { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        echo "Neovim's niri and Quickshell, a YubiKey and the TPM, in monospace: a speling slip." > notes.md
        ${probed} 'as designed' notes.md -c 'luafile ${probe}'
        touch $out
      '';

  # Neovim says each file it opens to the desktop's list of files opened
  # lately, through recent-files where the desktop has it, stood in here:
  # each as it comes into view, going back to one too, and nothing that
  # isn't a file on disk, such as its help.
  neovim-says-the-files-it-opens =
    let
      home = account workstation;
      # It keeps what it was asked, so a call it refuses reads as it was.
      recent-files = pkgs.writeShellScriptBin "recent-files" ''
        [ "$1 $2" = "add nvim" ] || { echo "recent-files $*" >> "$SAID"; exit 1; }
        shift 2
        printf '%s\n' "$@" >> "$SAID"
      '';
      heard = "vim.fn.filereadable(vim.env.SAID) == 1 and #vim.fn.readfile(vim.env.SAID) >= 3";
      said = "vim.wait(10000, function() return ${heard} end); io.stdout:write(vim.fn.filereadable(vim.env.SAID) == 1 and table.concat(vim.fn.readfile(vim.env.SAID), ' ') or 'nothing said')";
    in
    pkgs.runCommandLocal "neovim-says-the-files-it-opens"
      {
        nativeBuildInputs = [
          recent-files
          home.home.path
        ];
      }
      ''
        ${neovimAt home}
        export SAID=$PWD/said
        echo one > one.txt
        echo two > two.txt
        ${probed} "$PWD/one.txt $PWD/two.txt $PWD/one.txt" one.txt -c 'edit two.txt' -c 'help' \
          -c 'edit one.txt' -c "lua ${said}" -c 'qa!'
        touch $out
      '';

  # In a kitty window, the account's Neovim says where its own server
  # listens, once its terminal attaches, and clears it on leaving
  # (showing.lua); open-now asks it there what it has open, and hears both
  # files, the second in the tab it was opened in. Run in a terminal of its
  # own, as script gives it, which keeps what Neovim wrote to it.
  neovim-says-what-it-has-open =
    let
      home = account workstation;
      open-now = lib.getExe' (pkgs.callPackage ../open-with/recent-files.nix { }) "open-now";
      # Each file open, by its name, with the tabs showing it.
      reading =
        ''"\(.tabs) " + (.files | map((.path | ltrimstr($dir + "/")) + ":"''
        + ''+ (.tabs | map(tostring) | join(","))) | join(" "))'';
    in
    pkgs.runCommandLocal "neovim-says-what-it-has-open"
      {
        nativeBuildInputs = [
          home.home.path
          pkgs.util-linux
          pkgs.jq
        ];
      }
      ''
        ${neovimAt home}
        echo one > one.txt
        echo two > two.txt
        export KITTY_WINDOW_ID=1 TERM=xterm-256color
        # Flushed as Neovim writes it, since the check reads it while
        # Neovim runs, and a terminal drawn small writes too little to fill
        # script's buffer before it leaves.
        timeout 60 script -q -e -f -c \
          "nvim --cmd 'luafile ${./probes/says-what-it-has-open.lua}' one.txt" \
          written < /dev/null > /dev/null &
        running=$!
        said() {
          { grep -a -o $'\e]1337;SetUserVar=neovim_server[^\a]*' written 2> /dev/null || true; } \
            | sed 's/.*neovim_server//' | tr '\n' ' '
        }
        for _ in $(seq 600); do
          [ -n "$(said)" ] && break
          sleep 0.1
        done
        server=$(said | sed 's/^=//; s/ .*//' | base64 -d)
        held=$(${open-now} neovim at "$server" || true)
        touch asked
        wait "$running" || true
        open=$(printf %s "$held" | jq -r --arg dir "$PWD" '${reading}')
        [ "$open" = "2 one.txt:1 two.txt:2" ] || {
          echo "Neovim at '$server' said it holds '$held', read as '$open'"
          exit 1
        }
        wanted="=$(printf %s "$server" | base64 -w0)  "
        [ "$(said)" = "$wanted" ] || {
          echo "Neovim said '$(said)' to its kitty window, not '$wanted'"
          exit 1
        }
        touch $out
      '';

  # The account's Neovim binds every key in the map, and only those: each
  # action a key names is one a file registered, or one a plugin that
  # waits provides, each key is bound as it says, a scope's in a buffer it
  # arrives in, each plugin's window holds its keys and none of the
  # plugin's own, and no key Neovim has that says what it does is missing
  # from the map. Every plugin is loaded first, as it would be once used.
  neovim-binds-every-key =
    let
      home = account workstation;
      probe = ./probes/binds-every-key.lua;
    in
    pkgs.runCommandLocal "neovim-binds-every-key" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      # Neovim says where each key was set only when it's verbose, which
      # goes to a file rather than among the complaints.
      ${probed} 'every key' --cmd 'set verbose=1 verbosefile=verbose' \
        -c 'set verbose=0' -c 'luafile ${probe}' -c 'qa!'
      touch $out
    '';
}
