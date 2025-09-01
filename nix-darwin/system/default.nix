{ ... }:
{
  system.defaults = {
    CustomUserPreferences = {
      "com.apple.screencapture" = {
        location = "~/Pictures"; # Where Screen Captures Will Save To
      };
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
      show-recents = false;
    };
    finder = {
      AppleShowAllExtensions = true;
      FXPreferredViewStyle = "clmv";
      ShowHardDrivesOnDesktop = false;
      ShowStatusBar = false;
      ShowPathbar = true;
    };
    NSGlobalDomain = {
      AppleInterfaceStyle = "Dark";
      AppleShowAllExtensions = true;
      NSAutomaticCapitalizationEnabled = false;
      NSAutomaticPeriodSubstitutionEnabled = false;
      "com.apple.trackpad.forceClick" = true;
    };
    trackpad = {
      Clicking = false; # Tap to Click
      TrackpadThreeFingerDrag = false;
    };
  };
}

