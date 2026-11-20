#!/usr/bin/env bash
# Finds each finding key cadence's budget measures copy from spend-watch
# on both sides, so one renamed in spend-watch fails here rather than
# reading on the card as no budget unbilled or missed. Each is a whole
# line on both sides, so a test's fixture or a comment holding the same
# key can't stand in for one. It takes cadence's unbilled.rs and unresolved.rs,
# then spend-watch's collection.rs and detect.rs.
set -euo pipefail

unbilled=$1
unresolved=$2
collection=$3
detect=$4

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
line "$unresolved" 'const UNJUDGED: [&str; 1] = ["held"];'
line "$collection" '        format!("finding:{}", finding.key),'
line "$collection" '    let mut made = line("held", "Watch on hold", "newer history");'
line "$detect" '        format!("unbilled:{budget}"),'
line "$detect" 'pub const UNBILLED_UNSETTLED: &str = "unbilled-unsettled";'
line "$detect" '        format!("{UNBILLED_UNSETTLED}:{feed}"),'
line "$detect" 'pub const UNBILLED_HELD: &str = "unbilled-held";'
line "$detect" '        format!("{UNBILLED_HELD}:{feed}"),'
line "$detect" 'pub const UNRESOLVED: &str = "unresolved";'
line "$detect" '        format!("{UNRESOLVED}:{budget}"),'
