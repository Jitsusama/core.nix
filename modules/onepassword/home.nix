# 1Password's keys, for the launcher to list over its window, whose app id
# is com.onepassword.OnePassword. 1Password takes no settings for its keys,
# so these are its own, from its help's list for Linux. The system's half,
# the app itself, is nixos.nix beside this.
_:
let
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
}
