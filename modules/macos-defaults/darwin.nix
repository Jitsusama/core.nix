# The Mac's own settings that differ from Apple's: its shortcuts, keyboard
# repeat and the like, written as the defaults macOS reads.
let
  repeat = import ../keyboard/repeat.nix;
in
{
  system.defaults = {
    CustomUserPreferences = {
      "com.apple.symbolichotkeys" = {
        AppleSymbolicHotKeys = {
          "60".enabled = false; # Show Spotlight Search (Ctrl+Space)
          "61".enabled = false; # Show Finder Search (Ctrl+Opt+Space)
          "64".enabled = false; # Spotlight Menu (Cmd+Space)
        };
      };
    };
    dock = {
      autohide = true;
      mru-spaces = false;
      magnification = false;
      minimize-to-application = true;
      orientation = "bottom";
      show-recents = false;
      tilesize = 64;
      wvous-bl-corner = 4; # Show Desktop
      wvous-br-corner = 12; # Notification Centre
    };
    finder = {
      AppleShowAllExtensions = true;
      FXPreferredViewStyle = "clmv";
      NewWindowTarget = "Home";
      ShowHardDrivesOnDesktop = false;
      ShowStatusBar = false;
      ShowPathbar = true;
      _FXSortFoldersFirst = true;
    };
    hitoolbox.AppleFnUsageType = "Change Input Source";
    loginwindow.GuestEnabled = false;
    menuExtraClock = {
      Show24Hour = true;
      ShowDate = 0; # When Space Allows
    };
    NSGlobalDomain = {
      AppleICUForce24HourTime = true;
      AppleInterfaceStyle = "Dark";
      AppleShowAllExtensions = true;
      # macOS counts both in steps of 15 ms: the wait before the first
      # repeat, and the wait between repeats.
      InitialKeyRepeat = builtins.floor (repeat.delay / 15.0 + 0.5);
      KeyRepeat = builtins.floor (1000.0 / repeat.rate / 15 + 0.5);
      NSAutomaticCapitalizationEnabled = false;
      NSAutomaticPeriodSubstitutionEnabled = false;
      "com.apple.trackpad.forceClick" = true;
    };
    screencapture = {
      location = "~/Pictures";
      target = "clipboard";
    };
    trackpad = {
      Clicking = false; # Tap to Click
      TrackpadThreeFingerDrag = false;
    };
  };
}
