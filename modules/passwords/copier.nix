# Passwords' copier: the wl-copy the daemon puts a secret on the clipboard
# through, marked sensitive, which niri keeps from every fenced agent. It's
# patched to hold a sensitive copy in its own locked memory, rather than in
# a temp file any of the account's processes could open and served by a cat
# found on PATH (wl-copy-keeps-a-secret-in-memory.patch).
{ wl-clipboard }:
wl-clipboard.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [ ./wl-copy-keeps-a-secret-in-memory.patch ];
})
