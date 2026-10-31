# 1Password's keys, for the launcher to list over its window, whose app id
# is com.onepassword.OnePassword. 1Password takes no settings for its keys,
# so these are its own, from its help's list for Linux. Its desktop entry is
# its package's, with two things it can be asked from outside its window,
# which the launcher lists among its own actions: Quick Access, and Lock
# 1Password, named as its key list names Ctrl+Shift+L. The system's half,
# the app itself, is nixos.nix beside this. Where the system has the app, it
# starts with the session, without its window, as its own Start at Login
# would, so its command-line tool and Chrome's extension find it after a
# boot.
{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
let
  app = osConfig.programs._1password-gui or { };

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
  imports = [ ../keys/home.nix ];

  jitsusama.keys.onepassword = keys;

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
