# 1Password's keys, for the launcher to list over its window, whose app id
# is com.onepassword.OnePassword. 1Password takes no settings for its keys,
# so these are its own, from its help's list for Linux. Its desktop entry is
# its package's, with two things it can be asked from outside its window,
# which the launcher lists among its own actions: Quick Access, and Lock
# 1Password, named as its key list names Ctrl+Shift+L. The system's half,
# the app itself, is nixos.nix beside this. Where the system has the app, it
# starts with the session, without its window, as its own Start at Login
# would, so its command-line tool and Chrome's extension find it after a
# boot. There, 1Password is also a password manager of Passwords
# (../passwords), read through its command-line tool by manager/.
#
# Where the system has op, zsh completes it with op's own completion, as
# `op completion zsh` printed it when nixpkgs built the op the system
# wraps: a file of op's package, so nothing runs op to make it, here or on
# the machine. The system's profile links it too, but only where the
# system links zsh's files; given here, it comes with op wherever op does,
# and its build fails if a release stops shipping it. It's cobra's: each
# Tab runs `op __complete` for what to offer.
#
# 1Password's shell plugins are left out. Each is made by `op plugin init
# <tool>`, which asks 1Password which item holds the tool's credentials
# and writes the answer, naming that item, to ~/.config/op/plugins/, with
# an alias that runs the tool through `op plugin run` in plugins.sh beside
# it. That file is op's own state, chosen in op's prompt, which no module
# may write; the aliases alone would have each tool stop on its every run
# to ask in the terminal which item to use, with 1Password asking Joel to
# let op in; and they'd put op before tools that sign in on their own
# today, gh among them.
{ passwords }:
{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
let
  app = osConfig.programs._1password-gui or { };
  cli = osConfig.programs._1password or { };
  # 1Password's command-line tool as the system wraps it, in the group
  # 1Password's app trusts to ask it (nixos.nix).
  op = "${osConfig.security.wrapperDir or "/run/wrappers/bin"}/op";
  manager = pkgs.callPackage ./manager/package.nix { };
  # op's completion, as op's package has it.
  opCompletion = pkgs.runCommandLocal "op-zsh-completion" { } ''
    install -D -m 444 ${cli.package}/share/zsh/site-functions/_op \
      $out/share/zsh/site-functions/_op
    grep -q '^#compdef op$' $out/share/zsh/site-functions/_op
  '';

  grouped = group: map (key: key // { inherit group; });

  keys =
    grouped "finding" [
      {
        chord = "Ctrl+f";
        does = "Search every item";
      }
      {
        chord = "Ctrl+Alt+f";
        does = "Find in this list";
      }
      {
        chord = "Escape";
        does = "Clear the search";
      }
      {
        chord = "Ctrl+1";
        does = "Every account";
      }
      {
        chord = "Alt+Left";
        does = "Back in 1Password";
      }
      {
        chord = "Alt+Right";
        does = "Forward in 1Password";
      }
    ]
    ++ grouped "item" [
      {
        chord = "Ctrl+c";
        does = "Copy the username";
      }
      {
        chord = "Ctrl+Shift+c";
        does = "Copy the password";
      }
      {
        chord = "Ctrl+Alt+c";
        does = "Copy the one-time password";
      }
      {
        chord = "Ctrl+Shift+f";
        does = "Open the site and sign in";
      }
      {
        chord = "Ctrl+r";
        does = "Show or hide the secrets";
      }
      {
        chord = "Ctrl+e";
        does = "Edit the item";
      }
      {
        chord = "Ctrl+s";
        does = "Save the item";
      }
      {
        chord = "Ctrl+o";
        does = "The item in a window of its own";
      }
      {
        chord = "Ctrl+n";
        does = "A new item";
      }
      {
        chord = "Delete";
        does = "Archive the item";
      }
    ]
    ++ [
      {
        chord = "Ctrl+Shift+d";
        does = "Show or hide the sidebar";
      }
      {
        chord = "Ctrl+Shift+l";
        does = "Lock 1Password";
      }
      {
        chord = "Ctrl+Shift+/";
        does = "1Password's own list of keys";
      }
    ];
in
{
  imports = [
    ../keys/home.nix
    passwords
  ];

  jitsusama.keys.onepassword = keys;

  home.packages = lib.optional (cli.enable or false) opCompletion;

  # The first password manager, wherever the system has 1Password's app
  # and the command-line tool that asks it.
  jitsusama.passwords.managers =
    lib.mkIf ((app.enable or false) && (osConfig.programs._1password.enable or false))
      {
        "1password" = {
          order = 10;
          command = [
            (lib.getExe manager)
            "--op"
            op
          ];
          program = "com.onepassword.OnePassword";
          called = "1Password";
          # 1Password's own link to an item, which its desktop entry takes
          # as the onepassword scheme's handler, opening the item there.
          shows = "onepassword://view-item?a={account}&v={collection}&i={id}";
        };
      };

  systemd.user.services.onepassword = lib.mkIf (app.enable or false) {
    Unit = {
      Description = "1Password, started with the session without its window";
      After = [ "graphical-session.target" ];
      PartOf = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe' app.package "1password")
        "--silent"
      ];
      # Quitting it from its own menu ends it for the session; a crash
      # starts it again.
      Restart = "on-failure";
      Slice = "app.slice";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  # The package's entry as it has it, which this one shadows, the account's
  # folder coming first. Its actions are written in their names' order, so
  # Quick Access, which leaves 1Password as it is, comes before Lock
  # 1Password, which changes it.
  xdg.desktopEntries = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    "com.onepassword.OnePassword" = {
      name = "1Password";
      comment = "Password manager and secure wallet";
      exec = "1password %U";
      icon = "1password";
      categories = [ "Office" ];
      mimeType = [
        "x-scheme-handler/onepassword"
        "x-scheme-handler/onepassword8"
      ];
      settings.StartupWMClass = "com.onepassword.OnePassword";
      actions = {
        access = {
          name = "Quick Access";
          exec = "1password --quick-access";
        };
        lock = {
          name = "Lock 1Password";
          exec = "1password --lock";
        };
      };
    };
  };
}
