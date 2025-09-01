{ ... }:
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
      Show24Hours = true;
      ShowDate = 0; # When Space Allows
    };
    NSGlobalDomain = {
      AppleICUForce24HourTime = true;
      AppleInterfaceStyle = "Dark";
      AppleShowAllExtensions = true;
      KeyRepeat = 2;
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
