# pi, as the shell opens it, for the VM tests, which have no pi: where it
# started and the quest it was told to load, then what it was told, a line
# each, written down. While a test leaves pi-stays, it stays, its window
# titled with all it was told, so the test can find where it opened.
{ pkgs }:
pkgs.writeShellApplication {
  name = "pi";
  text = ''
    echo "$PWD ''${QUEST_WORKFLOW_AUTOLOAD_ID:-}" > "$XDG_RUNTIME_DIR/pi-opened"
    printf '%s\n' "$@" >> "$XDG_RUNTIME_DIR/pi-opened"
    if [ -e "$XDG_RUNTIME_DIR/pi-stays" ]; then
      printf '\033]2;pi %s\007' "$*"
      exec sleep 600
    fi
  '';
}
