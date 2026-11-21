#!/usr/bin/env bash
# Finds each key cadence copies from spend-watch on both sides: the finding
# keys its budget measures match, and the fields of budgets.json its budget
# alarm reads. One renamed in spend-watch fails here rather than reading
# on the card as no budget unbilled or missed, or as no budget near its
# allowance. Each is a whole line on both sides, so a test's fixture or a
# comment holding the same key can't stand in for one, and a field is
# found in its own struct, as two of spend-watch's share a field's name.
# A field's serde attributes, and its struct's, are pinned with it, as
# serde can rename the key cadence reads without touching the field, and
# a serde attribute the check can't read whole, as one split over lines
# or inside another, fails it rather than reading as none.
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
# line that closes it, with the serde attributes just above it, each ended
# by a semicolon; an empty line stands for the struct's own opening line.
# A comment or a blank line between an attribute and its line keeps it.
within() {
  local got rc=0
  got=$(awk -v open="pub struct $2 {" -v want="$3" '
    function said() { printf "found:%s", attrs; found = 1; exit }
    /serde\(/ && !/^[[:space:]]*(\/\/|#\[serde\(.*\)\][[:space:]]*$)/ { unread = 1; exit }
    $0 == open { if (want == "") said(); inside = 1; attrs = ""; next }
    inside && /^}/ { exit }
    /^[[:space:]]*(#\[|\/\/|$)/ {
      if ($0 ~ /^[[:space:]]*#\[serde/) { a = $0; sub(/^[[:space:]]+/, "", a); attrs = attrs a ";" }
      next
    }
    inside && $0 == want { said() }
    { attrs = "" }
    END { if (unread) exit 4; if (!found) exit 3 }' "$1") || rc=$?
  if ((rc == 4)); then
    echo "$1 has a serde attribute this check can't read whole, split or inside another" >&2
    exit 1
  elif ((rc != 0)); then
    echo "$1's $2 no longer has the line: $3" >&2
    exit 1
  fi
  [[ $got == "found:${4-}" ]] || {
    echo "$1's $2 has serde attributes '${got#found:}' on '${3:-its opening}', not '${4-}'" >&2
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

# budgets.json, written whole from spend-watch's Standing, of the version
# cadence reads.
line "$status" '        let path = runtime.join("budgets.json");'
line "$status" '        budgets: Option<&Standing>,'
line "$status" \
  '            let text = serde_json::to_string(budgets).map_err(|error| error.to_string())?;'
line "$standing" 'pub const VERSION: u32 = 1;'
line "$budgets" 'const VERSION: u32 = 1;'
within "$standing" Standing ''
within "$standing" Standing '    pub v: u32,'
line "$standing" '        v: VERSION,'
line "$budgets" '    if file["v"] != VERSION {'
within "$standing" Standing '    pub at: String,'
line "$budgets" '    let written = file["at"]'
within "$standing" Standing '    pub budgets: Vec<Budget>,'
line "$budgets" '    Ok(file["budgets"]'
within "$standing" Budget ''
within "$standing" Budget '    pub name: String,'
within "$standing" Budget '    pub label: String,'
line "$budgets" '                        budget: text(&budget["name"]),'
line "$budgets" '                        label: text(&budget["label"]),'
within "$standing" Budget '    pub allowances: Vec<Allowance>,'
line "$budgets" '            budget["allowances"]'
within "$standing" Allowance '' '#[serde(rename_all = "camelCase")];'
within "$standing" Allowance '    pub name: String,'
within "$standing" Allowance '    pub label: String,'
line "$budgets" '                        name: text(&allowance["name"]),'
line "$budgets" '                        window: text(&allowance["label"]),'
within "$standing" Allowance '    pub nearest: Option<f64>,' \
  '#[serde(skip_serializing_if = "Option::is_none")];'
line "$budgets" '                        share: allowance["nearest"].as_f64()?,'
within "$standing" Allowance '    pub standing: String,' \
  '#[serde(skip_serializing_if = "String::is_empty")];'
line "$budgets" '                        standing: text(&allowance["standing"]),'
within "$standing" Allowance '    pub resets: Option<String>,' \
  '#[serde(skip_serializing_if = "Option::is_none")];'
line "$budgets" '                        resets: allowance["resets"].as_str().map(Into::into),'
