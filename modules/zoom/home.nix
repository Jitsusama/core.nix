# Zoom, for meetings, on Linux: its own client where nixpkgs builds one
# for the machine, and elsewhere, as on an Apple laptop running Linux, its
# web client in a window of its own, as Omarchy opens it there. Zoom's own
# links, which a meeting's page hands the browser, join the meeting in the
# web client (opens.sh), so a link from mail or a calendar still joins.
# Which browser makes the window is the machine's to choose, so
# jitsusama.zoom.window says. A Mac gets Zoom's client from elsewhere,
# such as Homebrew, so nothing here applies to one.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  client = lib.meta.availableOn pkgs.stdenv.hostPlatform pkgs.zoom-us;

  # The window command, each %u in it the link, as a desktop entry writes it.
  window = pkgs.writeShellScript "zoom-window" ''
    exec ${
      lib.concatMapStringsSep " " (
        word: lib.concatStringsSep ''"$1"'' (map lib.escapeShellArg (lib.splitString "%u" word))
      ) config.jitsusama.zoom.window
    }
  '';

  opens = pkgs.writeShellScriptBin "zoom-opens" ''
    exec ${lib.getExe pkgs.bash} ${./opens.sh} ${window} "$@"
  '';
in
{
  options.jitsusama.zoom.window = lib.mkOption {
    type = lib.types.nonEmptyListOf lib.types.str;
    default = [
      (lib.getExe' pkgs.xdg-utils "xdg-open")
      "%u"
    ];
    defaultText = lib.literalExpression ''[ "xdg-open" "%u" ]'';
    example = [
      "chromium"
      "--app=%u"
    ];
    description = ''
      The command that opens Zoom's web client in a window of its own, each
      %u in it the link, where Zoom builds no client for the machine. By
      default it opens in the default browser, as a tab where that's what
      it does.
    '';
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    if client then
      { home.packages = [ pkgs.zoom-us ]; }
    else
      {
        # Zoom as a program, and the one program that says it opens Zoom's
        # own links, so gio sends them here without a default naming it.
        xdg.desktopEntries.zoom = {
          name = "Zoom";
          comment = "Meetings in Zoom's web client";
          exec = "${lib.getExe opens} %u";
          categories = [ "Network" ];
          mimeType = [
            "x-scheme-handler/zoommtg"
            "x-scheme-handler/zoomus"
          ];
        };
      }
  );
}
