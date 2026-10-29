"""Leave a Chrome history caught halfway through a write, for checks.nix.

It writes 2000 pages titled Kept, then starts a write retitling every one,
with so little cache that SQLite spills the changed pages into the file
before the write is done, and copies the file and its journal into the
profile folder given while the write is still open, as Chrome dying then
would leave them. Its exit leaves the write unfinished.

usage: mid-write.py PROFILE
"""

import os
import shutil
import sqlite3
import sys
from pathlib import Path

PAGES = 2000


def main() -> None:
    """Write the history, start the write and copy both files halfway."""
    profile = Path(sys.argv[1])
    history = Path("History")
    connection = sqlite3.connect(history, isolation_level=None)
    connection.execute("PRAGMA journal_mode=DELETE")
    connection.execute(
        "CREATE TABLE urls(id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR,"
        " visit_count INTEGER DEFAULT 0, typed_count INTEGER DEFAULT 0,"
        " last_visit_time INTEGER NOT NULL, hidden INTEGER DEFAULT 0)",
    )
    connection.executemany(
        "INSERT INTO urls(url, title, last_visit_time) VALUES (?, ?, ?)",
        [(f"https://site{page}.example.org/", "Kept", page) for page in range(PAGES)],
    )
    connection.execute("PRAGMA cache_size=1")
    connection.execute("BEGIN")
    connection.execute(
        "UPDATE urls SET title = 'Half written ' || id || ' ' || hex(randomblob(64))",
    )
    journal = Path("History-journal")
    if not journal.exists():
        sys.exit("no journal while the write is open, so nothing is halfway")
    shutil.copy(history, profile / "History")
    shutil.copy(journal, profile / "History-journal")
    # The file alone must hold half-written pages, or nothing needs rolling
    # back and the check would prove nothing.
    shutil.copy(history, "spilled")
    spilled = sqlite3.connect("file:spilled?mode=ro", uri=True)
    half = spilled.execute("SELECT count(*) FROM urls WHERE title <> 'Kept'").fetchone()[0]
    if half == 0:
        sys.exit("the write never reached the file, so nothing is halfway")
    # Leave as a crash would, the write unfinished.
    os._exit(0)


if __name__ == "__main__":
    main()
