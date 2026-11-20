#!/usr/bin/env bash
# Finds each finding key cadence's budget measures copy from spend-watch
# on both sides, so one renamed in spend-watch fails here rather than
# reading on the card as no budget unbilled or missed. Cadence's copies
# must be whole lines, so a test's fixture holding the same key can't
# stand in for one. It takes cadence's unbilled.rs and unresolved.rs,
# then spend-watch's collection.rs and detect.rs.
set -euo pipefail

unbilled=$1
unresolved=$2
collection=$3
detect=$4

has() {
  grep -qF -- "$2" "$1" || {
    echo "$1 no longer holds: $2" >&2
    exit 1
  }
}

line() {
  grep -qxF -- "$2" "$1" || {
    echo "$1 no longer has the line: $2" >&2
    exit 1
  }
}

line "$unbilled" 'const FINDING: &str = "finding:unbilled:";'
line "$unbilled" '    "held",'
line "$unbilled" '    "finding:unbilled-unsettled:",'
line "$unbilled" '    "finding:unbilled-held:",'
line "$unresolved" 'const FINDING: &str = "finding:unresolved:";'
has "$collection" 'format!("finding:{}", finding.key)'
has "$collection" 'line("held", "Watch on hold"'
has "$detect" 'format!("unbilled:{budget}")'
has "$detect" 'pub const UNBILLED_UNSETTLED: &str = "unbilled-unsettled";'
has "$detect" 'format!("{UNBILLED_UNSETTLED}:{feed}")'
has "$detect" 'pub const UNBILLED_HELD: &str = "unbilled-held";'
has "$detect" 'format!("{UNBILLED_HELD}:{feed}")'
has "$detect" 'pub const UNRESOLVED: &str = "unresolved";'
has "$detect" 'format!("{UNRESOLVED}:{budget}")'
