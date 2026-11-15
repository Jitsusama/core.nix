# shellcheck shell=bash
# Stands in for `cadence axes --served WORDS` in the launcher's Spec check
# (tests/spec-served.qml): two axes as cadence words them, Time to Done out
# of spec with one reading inside it, and Quality in spec, and the head
# saying one is out. Its arguments are cadence's, and read by nobody here.
cat <<'EOF'
{
  "head": {"key": "head:spec", "place": "spec", "said": "1 reading out of spec", "glyph": "failed"},
  "rows": [
    {
      "key": "row:spec:time-to-done", "thing": "axis:time-to-done", "place": "spec",
      "kind": "axis", "depth": 0, "name": "Time to Done", "glyph": "failed",
      "side": [{"said": "1 out"}], "acts": [{"key": "verb:more", "step": "rows"}],
      "drawer": {"eyebrow": "from cadence \u00b7 time to done", "title": "Time to Done",
        "facts": [{"name": "Your Use Out", "said": "1 of 1"}]}
    },
    {
      "key": "row:spec:time-to-done:0", "thing": "axis:time-to-done:0", "place": "spec",
      "kind": "reading", "depth": 1, "parent": "row:spec:time-to-done",
      "name": "merge to on 53 min, at most 18 min", "glyph": "failed",
      "side": [{"said": "Integration"}], "acts": [],
      "drawer": {"eyebrow": "from cadence \u00b7 time to done",
        "title": "merge to on 53 min, at most 18 min", "facts": []}
    },
    {
      "key": "row:spec:quality", "thing": "axis:quality", "place": "spec",
      "kind": "axis", "depth": 0, "name": "Quality",
      "side": [{"said": "in spec"}], "acts": [{"key": "verb:more", "step": "rows"}],
      "drawer": {"eyebrow": "from cadence \u00b7 quality", "title": "Quality", "facts": []}
    }
  ]
}
EOF
