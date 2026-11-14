# The checks of keys, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
let
  # A chord of a made-up key map, for the controls below.
  bind = chord: keys: act: {
    inherit chord keys act;
    does = "Something";
    when = "";
    run = [ ];
  };
  keyMap = name: layers: pkgs.writeText "${name}.json" (builtins.toJSON layers);

  # Each way of straying the check has to catch, with what it says.
  controls = [
    {
      layers = keyMap "another-acts-key" [
        {
          name = "reader";
          kinds = [ "player" ];
          binds = [
            (bind "q" "q" "close")
            (bind "Space ?" "space ?" "show-keys")
            (bind "d d" "d d" "")
          ];
        }
      ];
      says = "reader: d d is its own (Something), but the language gives it to ending";
    }
    {
      layers = keyMap "an-act-left-out" [
        {
          name = "reader";
          kinds = [ "player" ];
          binds = [ (bind "q" "q" "close") ];
        }
      ];
      says = "reader: a player has show-keys, which it leaves out";
    }
    {
      layers = keyMap "not-the-acts-key" [
        {
          name = "reader";
          kinds = [ "player" ];
          binds = [
            (bind "q" "q" "close")
            (bind "Space h" "space h" "show-keys")
          ];
        }
      ];
      says = "reader: Space h isn't one of show-keys's keys";
    }
    {
      layers = keyMap "neovim-moves-a-key" [
        {
          name = "neovim";
          binds = [ (bind "Space h" "space h" "show-keys") ];
        }
      ];
      says = "neovim: Space h isn't one of show-keys's keys";
    }
    {
      layers = keyMap "neovim-closes-on-q" [
        {
          name = "neovim";
          binds = [ (bind "q" "q" "close") ];
        }
      ];
      says = "neovim: q isn't one of close's keys";
    }
    {
      layers = keyMap "neovim-takes-a-command-line-key" [
        {
          name = "neovim";
          binds = [ (bind "Ctrl+Y" "ctrl+y" "" // { when = "on the command line"; }) ];
        }
      ];
      says = "neovim: Ctrl+Y is its own (Something), but the language gives it to take-name";
    }
    {
      layers = keyMap "neovim-gives-a-key-away" [
        {
          name = "neovim";
          binds = [ (bind "Space ?" "space ?" "") ];
        }
      ];
      says = "neovim: Space ? is its own (Something), but the language gives it to show-keys";
    }
    # In a modal layer's normal mode an arrow is only its letter's motion,
    # alone: Down is j's act and nothing else's.
    {
      layers = keyMap "a-modal-layer-binds-an-arrow" [
        {
          name = "swayimg";
          binds = [ (bind "Down" "down" "") ];
        }
      ];
      says = "swayimg: Down gives an arrow its own act, where an arrow is only its letter's motion";
    }
    {
      layers = keyMap "an-arrow-for-another-letter" [
        {
          name = "mpv";
          binds = [ (bind "Down" "down" "up") ];
        }
      ];
      says = "mpv: Down gives an arrow to up, where an arrow is only its letter's motion";
    }
    {
      layers = keyMap "an-arrow-held" [
        {
          name = "zathura";
          binds = [ (bind "Shift+Down" "shift+down" "") ];
        }
      ];
      says = "zathura: Shift+Down gives an arrow its own act, where an arrow is only its letter's motion";
    }
    # Escape never closes or quits anything of ours (Joel, 2026-11-05).
    {
      layers = keyMap "escape-closes" [
        {
          name = "inlyne";
          binds = [ (bind "Escape" "escape" "" // { does = "Close the reader"; }) ];
        }
      ];
      says = ''inlyne: Escape does "Close the reader", where Escape never closes or quits'';
    }
  ];

  # Each way the language itself can stray, as acts added to it, with what
  # the check has to say.
  languageStrays = [
    {
      name = "a-short-form-shared";
      adds = ''
        [acts.stray-one]
        command = "strayone"
        short = "stray"
        kinds = []

        [acts.stray-two]
        command = "straytwo"
        short = "stray"
        kinds = []
      '';
      says = "the language gives :stray to both stray-one and stray-two";
    }
    {
      name = "a-name-neovim-has";
      adds = ''
        [acts.stray]
        command = "lock"
        short = "lock"
        kinds = []
      '';
      says = "stray's :lock is Neovim's :lockmarks";
    }
    {
      name = "a-short-form-neovim-has";
      adds = ''
        [acts.stray]
        command = "lockscreen"
        short = "lo"
        kinds = []
      '';
      says = "stray's :lo is Neovim's :loadview";
    }
    {
      name = "a-vim-command-shortened-otherwise";
      adds = ''
        [acts.stray]
        command = "registers"
        short = "regis"
        kinds = []
      '';
      says = "stray's :registers is Neovim's, whose short form is reg, not regis";
    }
    {
      name = "a-short-form-of-something-else";
      adds = ''
        [acts.stray]
        command = "stray"
        short = "sx"
        kinds = []
      '';
      says = "stray's short form sx doesn't begin its command, stray";
    }
    {
      name = "a-command-with-no-short-form";
      adds = ''
        [acts.stray]
        command = "stray"
        kinds = []
      '';
      says = "stray's command, stray, has no short form";
    }
    {
      name = "a-change-the-find-reaches";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        modes = ["find"]
        kinds = []
        changes = true
      '';
      says = "stray changes things, but the find line reaches it by ctrl+x";
    }
    # An act that ends, takes away or reaches someone never runs on Return
    # (VKCF3Z's spec, Return), and h, l and Backspace mean nothing on a
    # list (Joel, 2026-11-05: "where in vi does that step into shit?").
    {
      name = "an-end-that-changes-nothing";
      adds = ''
        [acts.stray]
        command = "strayone"
        short = "stray"
        kinds = []
        ends = true
      '';
      says = "stray ends something, but changes nothing";
    }
    {
      name = "an-end-on-return";
      adds = ''
        [acts.stray]
        keys = ["<CR>"]
        modes = ["visual"]
        kinds = []
        changes = true
        ends = true
      '';
      says = "stray ends something, but Return reaches it by enter in visual mode";
    }
    # Nor does one that calls someone or starts something new, as :meet
    # and :share do (VKCF3Z, 2026-11-05).
    {
      name = "a-start-that-changes-nothing";
      adds = ''
        [acts.stray]
        command = "strayone"
        short = "stray"
        kinds = []
        starts = true
      '';
      says = "stray starts something, but changes nothing";
    }
    {
      name = "a-start-on-return";
      adds = ''
        [acts.stray]
        keys = ["<CR>"]
        modes = ["visual"]
        kinds = []
        changes = true
        starts = true
      '';
      says = "stray starts something, but Return reaches it by enter in visual mode";
    }
    {
      name = "a-letter-stepping-in";
      adds = ''
        [acts.stray]
        keys = ["l"]
        modes = ["visual"]
        kinds = ["list"]
      '';
      says = "stray takes l on a list, where h, l and Backspace mean nothing";
    }
    {
      name = "an-arrow-of-its-own";
      adds = ''
        [acts.stray]
        keys = ["<Left>"]
        modes = ["visual"]
        kinds = ["page"]
      '';
      says = "stray takes left in visual mode, where an arrow is only its letter's motion";
    }
    {
      name = "an-arrow-in-a-sequence";
      adds = ''
        [acts.stray]
        keys = ["g<Right>"]
        kinds = ["list"]
      '';
      says = "stray takes g right in normal mode, where an arrow is only its letter's motion";
    }
    {
      name = "an-arrow-held";
      adds = ''
        [acts.stray]
        keys = ["<C-Down>"]
        kinds = ["list"]
      '';
      says = "stray takes ctrl+down in normal mode, where an arrow is only its letter's motion";
    }
    {
      name = "escape-given-away";
      adds = ''
        [acts.stray]
        keys = ["<Esc>"]
        modes = ["visual"]
        kinds = ["page"]
      '';
      says = "stray takes escape, where Escape only steps back, as Vim's does";
    }
    {
      name = "a-mode-unnamed";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        modes = ["operator"]
        kinds = []
      '';
      says = "stray is pressed in operator mode, which the language doesn't name";
    }
    {
      name = "a-kind-unnamed";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = ["widget"]
      '';
      says = "stray belongs to a widget, which the language doesn't name";
    }
    {
      name = "an-undoing-of-nothing";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        undoable = true
      '';
      says = "stray can be undone, but changes nothing";
    }
    {
      name = "a-guard-on-an-undoing";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        changes = true
        undoable = true
        guarded = true
      '';
      says = "stray guards itself, but it never asks, since it can be undone";
    }
    {
      name = "a-guard-on-nothing";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        guarded = true
      '';
      says = "stray guards itself, but it never asks, since it changes nothing";
    }
    {
      name = "a-cost-on-nothing";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        costly = true
      '';
      says = "stray costs a lot, but it changes nothing";
    }
    {
      name = "a-cost-on-an-undoing";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        changes = true
        undoable = true
        costly = true
      '';
      says = "stray costs a lot, but it can be undone";
    }
    {
      name = "a-cost-on-a-guard";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        changes = true
        guarded = true
        costly = true
      '';
      says = "stray costs a lot, but it guards itself";
    }
    {
      name = "a-field-unknown";
      adds = ''
        [acts.stray]
        keys = ["<C-x>"]
        kinds = []
        ranged = true
      '';
      says = "stray has ranged, which the language doesn't know";
    }
  ];
in
{
  # Every surface of ours speaks the key language (language.toml), and
  # Neovim's own keys are what the language is checked against: the key map
  # every machine writes is read as the launcher reads it, and a chord the
  # language gives to another act, an act a surface's kinds hold that it
  # leaves out, or a chord bound to an act that isn't one of the act's keys
  # fails (speaks-the-language.py), as does a language whose commands
  # Neovim names otherwise, by what the machine's Neovim resolves each
  # name to, read once as a file of its own. In normal and visual mode an
  # arrow is its letter's motion and nothing else, alone, on every layer of
  # ours, his Neovim's too; an arrow after the leader overlays Vim's keys
  # (Joel, 2026-11-05). Escape only steps back, as Vim's, and no layer of
  # ours closes or quits by it (Joel, 2026-11-05: "fuck escape quitting");
  # his Neovim keeps its own. So the check can't pass by
  # seeing nothing, each way of straying is shown to fail first, as is a
  # language that gives one key to two acts and the reader leaving out a
  # key it takes only while selecting.
  keys-speak-the-language =
    let
      # A workstation with a desktop has every layer, Neovim's among them.
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
      ];
      keys = (account machine).xdg.configFile."jitsusama/keys.json".source;
      # Every name the machine's Neovim takes for a command of its own,
      # looked up once with no config and kept, so the check reads a file
      # in a moment.
      neovimCommands = pkgs.runCommandLocal "neovim-commands.json" { } ''
        HOME=$TMPDIR ${lib.getExe (account machine).programs.neovim.package} --clean \
          -l ${./neovim-commands.lua} > $out
      '';
      check = "${lib.getExe pkgs.python3} ${./speaks-the-language.py}";
    in
    pkgs.runCommandLocal "keys-speak-the-language" { } ''
      # A language and a key map that stray, and what the check must say.
      strays() {
        if ${check} "$1" "$2" neovim ${neovimCommands} > said; then
          echo "The key language check passed on $2, which strays."
          exit 1
        fi
        if ! grep -qF -- "$3" said; then
          echo "On $2 the key language check didn't say: $3"
          cat said
          exit 1
        fi
      }
      ${lib.concatMapStrings (control: ''
        strays ${./language.toml} ${control.layers} ${lib.escapeShellArg control.says}
      '') controls}
      ${lib.concatMapStrings (stray: ''
        cat ${./language.toml} ${pkgs.writeText "${stray.name}.toml" stray.adds} > ${stray.name}.toml
        strays ${stray.name}.toml ${keyMap "none" [ ]} ${lib.escapeShellArg stray.says}
      '') languageStrays}
      cat ${./language.toml} > twice.toml
      printf '\n[acts.twice]\nkeys = ["j"]\nkinds = []\n' >> twice.toml
      strays twice.toml ${keyMap "none" [ ]} \
        "the language gives j to both down and twice in normal mode"
      ${lib.getExe pkgs.jq} \
        'map(if .name == "reader" then .binds |= map(select(.act != "word-on")) else . end)' \
        ${keys} > unselecting.json
      strays ${./language.toml} unselecting.json \
        "reader: a page has word-on, which it leaves out"
      # An arrow after the leader overlays Vim's own keys, as his Neovim's
      # Space and an arrow do (Joel, 2026-11-05: "leader stuff can overlay
      # default stuff"), so a surface's act may take one.
      cat ${./language.toml} > leader.toml
      # In visual, where his window-right isn't, so it gives no key twice.
      printf '\n[acts.over]\nkeys = ["<Space><Right>"]\nmodes = ["visual"]\nkinds = ["list"]\n' \
        >> leader.toml
      ${check} leader.toml ${keys} neovim ${neovimCommands}
      # His Neovim's arrows move as its letters do.
      ${lib.getExe pkgs.jq} \
        'map(if .name == "neovim" then .binds += [{chord: "Down", keys: "down", does: "Down",
          when: "", run: [], act: "down"}] else . end)' ${keys} > neovim.json
      ${check} ${./language.toml} neovim.json neovim ${neovimCommands}
      # An arrow moving as its letter does, as Joel has it (2026-11-05: "In
      # normal mode, in a buffer, arrows move the cursor"), on any layer:
      # as the language's motion, or as the layer's own letter, as mpv's
      # seeking.
      ${check} ${./language.toml} ${
        keyMap "an-arrow-as-its-letter" [
          {
            name = "swayimg";
            binds = [ (bind "Down" "down" "down") ];
          }
          {
            name = "mpv";
            binds = [
              (bind "j" "j" "")
              (bind "Down" "down" "")
            ];
          }
        ]
      } neovim ${neovimCommands}
      ${check} ${./language.toml} ${keys} neovim ${neovimCommands}
      touch $out
    '';

  # A chord two layers both claim never reaches the lower one, so a machine
  # with one doesn't build. The clashes here are spelt the way each program
  # spells its chords, so the map has to see through the spelling: kitty's
  # ctrl+shift+t is Neovim's Ctrl+Shift+T, and its super+return is niri's
  # Super+Return. A sequence whose first key a layer above takes can never
  # start, so it's refused too, as is a layer claiming one chord twice. Neovim
  # and pi are never heard at once, so a chord both claim is no clash, and
  # nor is one the picture viewer and the terminal both claim, in windows of
  # their own; the desktop's is refused it as any layer's.
  a-claimed-chord-is-refused =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.keys.neovim = [
            {
              chord = "Ctrl+Shift+T";
              does = "Reopen a closed buffer";
            }
            {
              chord = "Ctrl+Shift+H g";
              does = "Go to the help";
            }
            {
              chord = "Ctrl+G";
              does = "Say where the file is";
            }
          ];
          home-manager.users.someone.jitsusama.keys.swayimg = [
            {
              chord = "Ctrl+Shift+T";
              does = "Turn the picture";
            }
            {
              chord = "Super+Return";
              does = "Show the picture whole";
            }
          ];
          home-manager.users.someone.jitsusama.kitty.keys = lib.mkAfter [
            {
              chord = "super+return";
              does = "Open a tab";
              action = "new_tab";
            }
            {
              chord = "ctrl+shift+c";
              does = "Copy again";
              action = "copy_to_clipboard";
            }
          ];
        }
      ];
      refusals = lib.concatMapStrings (check: check.message) (
        lib.filter (check: !check.assertion) (account machine).assertions
      );
      fine =
        lib.all (check: check.assertion)
          (account (bareNixos [
            self.nixosModules.workstation
            self.nixosModules.graphical
          ])).assertions;
    in
    holds "a-claimed-chord-is-refused"
      "A chord two layers claim, or one layer claims twice, wasn't refused."
      (
        fine
        && lib.hasInfix "ctrl+shift+t: terminal (Open a tab here) takes it from neovim (Reopen a closed buffer)" refusals
        && lib.hasInfix "ctrl+shift+h: terminal (Pick a word from the screen) takes it from neovim (Go to the help)" refusals
        && !lib.hasInfix "(Write the message in Neovim)" refusals
        && lib.hasInfix "super+enter: desktop (Open a terminal) takes it from terminal (Open a tab)" refusals
        && lib.hasInfix "ctrl+shift+c in terminal: Copy, Copy again" refusals
        && !lib.hasInfix "(Turn the picture)" refusals
        && lib.hasInfix "super+enter: desktop (Open a terminal) takes it from swayimg (Show the picture whole)" refusals
      );
}
