# The checks of screen-text, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. The
# reader's own check is dictation's, whose crate it's built in.
{
  account,
  graphical,
  holds,
  onLinux,
  pkgs,
  ...
}:
{
  # A switch restarts it after it reloads the units, so the one answering
  # the shell is never the one from before the switch.
  screen-text-switches-to-the-new-one =
    holds "screen-text-switches-to-the-new-one"
      "A switch stops screen-text before reloading, so the shell would ask the old one."
      (((account graphical).systemd.user.services.screen-text.Unit.X-SwitchMethod or null) == "restart");
}
// onLinux {
  # The daemon end to end, on the CPU, in the build's sandbox:
  # tests/a-screenshot-is-found-by-its-words.sh says what it proves.
  screen-text-finds-a-screenshot-by-its-words =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      models = pkgs.callPackage ./models.nix { };
      font = "${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSans.ttf";
    in
    pkgs.runCommand "screen-text-finds-a-screenshot-by-its-words"
      {
        nativeBuildInputs = [
          pkgs.imagemagick
          pkgs.jq
          pkgs.socat
        ];
      }
      ''
        $BASH ${./tests/a-screenshot-is-found-by-its-words.sh} \
          ${dictation}/bin/screen-text ${models} ${font}
        touch $out
      '';
}
