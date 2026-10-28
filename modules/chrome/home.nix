# Chrome's keys, for the launcher to list over its windows: the browser's
# own, and those of a window it opens for one site, such as Calendar or
# Mail, whose app id names the site (launcher/KeyGroups.js). Chrome comes
# from wherever the machine gets it, so this lists its keys and installs
# nothing; a machine with Chrome imports it. Chrome takes no settings for
# its keys, so these are its own, which mostly speak the keyboard's
# language already (docs/design.md, The Keyboard): T tabs, W closes, a
# number is that tab, and the arrows go back and forward.
{ lib, ... }:
let
  grouped = group: map (key: key // { inherit group; });

  # A window for one site has no tabs, so these work in the browser's own.
  tabs = map (key: key // { when = "in the browser"; }) (
    [
      {
        chord = "Ctrl+t";
        does = "A new tab";
      }
      {
        chord = "Ctrl+w";
        does = "Close the tab";
      }
      {
        chord = "Ctrl+Shift+t";
        does = "Open the tab closed last";
      }
      {
        chord = "Ctrl+Tab";
        does = "The next tab";
      }
      {
        chord = "Ctrl+Shift+Tab";
        does = "The tab before";
      }
    ]
    ++ map (number: {
      chord = "Ctrl+${toString number}";
      does = "Tab number ${toString number}";
    }) (lib.range 1 8)
    ++ [
      {
        chord = "Ctrl+9";
        does = "The last tab";
      }
    ]
  );

  keys =
    grouped "tabs" tabs
    ++ grouped "pages" [
      {
        chord = "Alt+Left";
        does = "Back a page";
      }
      {
        chord = "Alt+Right";
        does = "Forward a page";
      }
      {
        chord = "Ctrl+r";
        does = "Load the page again";
      }
      {
        chord = "Ctrl+l";
        does = "Type an address";
      }
      {
        chord = "Ctrl+f";
        does = "Find words on the page";
      }
      {
        chord = "Ctrl+Plus";
        does = "The page larger";
      }
      {
        chord = "Ctrl+Minus";
        does = "The page smaller";
      }
      {
        chord = "Ctrl+0";
        does = "The page at its own size";
      }
    ]
    ++ [
      {
        chord = "Ctrl+n";
        does = "A new browser window";
      }
      {
        chord = "Ctrl+Shift+n";
        does = "A private browser window";
      }
      {
        chord = "Ctrl+h";
        does = "The pages seen before";
      }
      {
        chord = "Ctrl+j";
        does = "The downloads";
      }
      {
        chord = "F11";
        does = "The page on the whole screen";
      }
    ];
in
{
  imports = [ ../keys/home.nix ];

  jitsusama.keys.chrome = keys;
}
