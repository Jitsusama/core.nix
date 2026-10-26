# The checks of keys, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
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
