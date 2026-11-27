# shellcheck shell=bash
# Stands in for `cadence axes --asked WORDS` in the launcher's Asked For
# check (modules/quickshell/tests/asked-served.qml). With ASKED_READ set to
# unread it fails as cadence does when it can't read the asks list, so
# cadence-spec heads Asked For unread in the launcher's words; otherwise it
# prints two asks as cadence words them, one late, and the head saying so.
# Asked for Spec's axes (--served), it has none, as Spec isn't what's
# checked here.
if [ "${2:-}" = --served ]; then
  echo '{"head": {"key": "head:spec", "place": "spec", "said": "in spec"}, "rows": []}'
  exit 0
fi
if [ "${ASKED_READ:-}" = unread ]; then
  echo "cadence: the asks list is unread, as it's a stand-in told so" >&2
  exit 1
fi
cat <<'EOF'
{
  "head": {"key": "head:asked", "place": "asked", "said": "1 late of 2", "glyph": "failed"},
  "rows": [
    {
      "key": "row:asked:be66afc9", "thing": "ask:be66afc9", "place": "asked", "kind": "ask",
      "name": "split the launcher's words", "hue": "alert",
      "side": [{"said": "late 2h"}], "acts": [],
      "drawer": {"eyebrow": "asked for", "title": "split the launcher's words", "facts": []}
    },
    {
      "key": "row:asked:dbb6623c", "thing": "ask:dbb6623c", "place": "asked", "kind": "ask",
      "name": "say what moved", "side": [{"said": "due Fri"}], "acts": [],
      "drawer": {"eyebrow": "asked for", "title": "say what moved", "facts": []}
    }
  ]
}
EOF
