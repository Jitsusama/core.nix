#!/usr/bin/env bash
# Finds each key cadence copies from spend-watch on both sides: the finding
# keys its budget measures match, and the fields of budgets.json its budget
# alarm reads. One renamed in spend-watch fails here rather than reading
# on the card as no budget unbilled or missed, or as no budget near its
# allowance. Each is a whole line on both sides, so a test's fixture or a
# comment holding the same key can't stand in for one, and a field is
# found in its own struct, as two of spend-watch's share a field's name.
# It takes cadence's unbilled.rs, unresolved.rs and watch/budgets.rs, then
# spend-watch's collection.rs, detect.rs, standing.rs and status.rs.
set -euo pipefail

unbilled=$1
unresolved=$2
budgets=$3
collection=$4
detect=$5
standing=$6
status=$7

line() {
  grep -qxF -- "$2" "$1" || {
    echo "$1 no longer has the line: $2" >&2
    exit 1
  }
}

# A line of one struct's body, between its opening line and the first
# line that closes it.
within() {
  local body
  body=$(awk -v open="pub struct $2 {" \
    '$0 == open {inside = 1; next} inside && /^}/ {exit} inside' "$1")
  grep -qxF -- "$3" <<<"$body" || {
    echo "$1's $2 no longer has the line: $3" >&2
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

# budgets.json, written whole from spend-watch's Standing.
line "$status" '        let path = runtime.join("budgets.json");'
within "$standing" Standing '    pub at: String,'
line "$budgets" '    let written = file["at"]'
within "$standing" Standing '    pub budgets: Vec<Budget>,'
line "$budgets" '    Ok(file["budgets"]'
within "$standing" Budget '    pub name: String,'
within "$standing" Budget '    pub label: String,'
line "$budgets" '                        budget: text(&budget["name"]),'
line "$budgets" '                        label: text(&budget["label"]),'
within "$standing" Budget '    pub allowances: Vec<Allowance>,'
line "$budgets" '            budget["allowances"]'
within "$standing" Allowance '    pub name: String,'
within "$standing" Allowance '    pub label: String,'
line "$budgets" '                        name: text(&allowance["name"]),'
line "$budgets" '                        window: text(&allowance["label"]),'
within "$standing" Allowance '    pub nearest: Option<f64>,'
line "$budgets" '                        share: allowance["nearest"].as_f64()?,'
within "$standing" Allowance '    pub standing: String,'
line "$budgets" '                        standing: text(&allowance["standing"]),'
within "$standing" Allowance '    pub resets: Option<String>,'
line "$budgets" '                        resets: allowance["resets"].as_str().map(Into::into),'
