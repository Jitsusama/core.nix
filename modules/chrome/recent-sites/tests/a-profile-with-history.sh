# shellcheck shell=bash
# Builds a Chrome profile with a history, for commands-agree to set
# chrome-recent-sites' runs at each width from: six sites visited, one
# twice, one titled past any width, one of Google's own windows and one
# sign-in page, both left out; and three downloads, two still where
# Chrome saved them, one with a long path, and one since removed. It's
# built rather than kept, since a kept SQLite file says nothing of what
# it holds. It needs sqlite3 on PATH.
# usage: a-profile-with-history.sh <folder to build it in>
set -euo pipefail

out=$1
mkdir -p "$out/Downloads/a folder with a long name for its reports"
head -c 2048 /dev/zero >"$out/Downloads/notes.txt"
head -c 1234567 /dev/zero \
  >"$out/Downloads/a folder with a long name for its reports/the quarterly report.pdf"

# Every visit is some seconds past one moment, 1700000000 after 1970, on
# Chrome's clock, which counts microseconds from 1601.
then=$(((1700000000 + 11644473600) * 1000000))
second=1000000

sqlite3 "$out/History" <<SQL
CREATE TABLE urls(id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR,
  visit_count INTEGER DEFAULT 0, typed_count INTEGER DEFAULT 0,
  last_visit_time INTEGER NOT NULL, hidden INTEGER DEFAULT 0);
INSERT INTO urls(url, title, last_visit_time) VALUES
  ('https://example.org/guide', 'A Guide to Everything', $((then + 900 * second))),
  ('https://example.org/older', 'An Older Page', $((then + 100 * second))),
  ('https://docs.example.net/book/chapter-twelve',
   'Chapter Twelve: Where the Long Title Runs On Past the Width of Any Terminal',
   $((then + 800 * second))),
  ('https://news.example.com/', 'The News', $((then + 700 * second))),
  ('https://mail.google.com/mail/u/0/', 'Inbox', $((then + 600 * second))),
  ('https://accounts.google.com/signin', 'Sign in', $((then + 500 * second))),
  ('https://shop.example.com/cart', 'Your Cart', $((then + 400 * second))),
  ('http://plain.example.com/', 'Plain', 0);
CREATE TABLE downloads (id INTEGER PRIMARY KEY, guid VARCHAR NOT NULL,
  current_path LONGVARCHAR NOT NULL, target_path LONGVARCHAR NOT NULL,
  start_time INTEGER NOT NULL, received_bytes INTEGER NOT NULL,
  total_bytes INTEGER NOT NULL, state INTEGER NOT NULL,
  end_time INTEGER NOT NULL, tab_url VARCHAR, referrer VARCHAR);
INSERT INTO downloads(guid, current_path, target_path, start_time, received_bytes,
  total_bytes, state, end_time, tab_url, referrer) VALUES
  ('a', '', '$out/Downloads/notes.txt', $((then + 300 * second)), 2048, 2048, 1,
   $((then + 305 * second)), 'https://example.org/guide', ''),
  ('b', '',
   '$out/Downloads/a folder with a long name for its reports/the quarterly report.pdf',
   $((then + 200 * second)), 1234567, 1234567, 1, $((then + 210 * second)), '',
   'https://docs.example.net/book/chapter-twelve'),
  ('c', '', '$out/Downloads/gone.zip', $((then + 100 * second)), 10, 10, 1,
   $((then + 101 * second)), 'https://shop.example.com/cart', '');
SQL
