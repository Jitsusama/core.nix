# xdg-open, handing what it's given to gio (nixos.nix says why).
{ glib, writeShellApplication }:
writeShellApplication {
  name = "xdg-open";
  runtimeInputs = [ glib.bin ];
  text = ''
    usage="usage: xdg-open { file | URL }, opened through gio"
    if [ "$#" -eq 0 ]; then
      echo "$usage" >&2
      exit 1
    fi
    case "$1" in
      --help | --manual | --version)
        echo "$usage"
        exit 0
        ;;
    esac
    exec gio open "$@"
  '';
}
