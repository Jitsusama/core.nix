# The comments naming a path gone before comments-name-what-exists, each as
# `file path`, once for each mention, waiting while its owner points it at
# where the thing is now: the stand-ins moved to tests/desktop/stand-ins/,
# and reader's place.rs became place/mod.rs. The check fails on a mention
# not listed, and on a line listed more often than it's found, so the list
# only shrinks.
[
  "modules/eero/src/rows_tests.rs tests/desktop/stand-in-eero.py"
  "modules/google/src/bin/google-rows.rs tests/desktop/stand-in-google.py"
  "tests/desktop/looks/desktop-looks-programs-vm-test.py modules/reader/src/place.rs"
]
