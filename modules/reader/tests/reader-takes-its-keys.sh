# shellcheck shell=bash
# Runs inside cage: opens a long markdown file in the reader, presses
# Neovim's keys through wtype and reads where each took the view from the
# reader's key log (READER_KEYS_LOG), photographs the top of the page once
# its heading is drawn, and closes it with :q. cage doesn't pass on how this
# ended, so it leaves ./passed once every key did what it should.
#
# cage has no keyboard until wtype makes one, so the reader has the
# keyboard only once the first key is typed: G is pressed until it's taken.
# Each wtype adds a keyboard of its own and takes it away as it ends, so
# each one waits before typing for the reader to take the new keyboard.
#
# usage: reader-takes-its-keys.sh <file> <photo> <heading colour> <cursor> <cursor text> <accent>
#
# The markdown a copy is held to holds backticks and dollars, meant as
# themselves, so it's quoted whole in single quotes.
# shellcheck disable=SC2016
set -euo pipefail

file=$1
photo=$2
strong=$3
cursor=$4
cursor_text=$5
accent=$6
: > said

READER_KEYS_LOG=1 reader "$file" > said 2>&1 &
reader=$!

fail() {
  echo "$1"
  cat said
  kill "$reader" 2> /dev/null || true
  exit 1
}

# Whether the log holds `count` lines matching `pattern`.
holds() { [ "$(grep -c -a -E "$1" said || true)" -ge "$2" ]; }

# Waits until the log holds `count` lines matching `pattern`, or 50 seconds
# pass, however quiet the log is meanwhile.
logged() {
  local deadline=$((SECONDS + 50))
  until holds "$1" "$2"; do
    ((SECONDS < deadline)) || fail "the reader never logged $2 of: $1"
    inotifywait -q -q -t 1 -e modify said || true
  done
}

# Where the view's top was after the last key, in points.
heading() { grep -a -E '^key ' said | tail -n 1 | sed -E 's/.*heading ([0-9.]+) pt/\1/'; }

# Where the cursor was after the last key, in points.
cursor() { grep -a -E '^cursor: ' said | tail -n 1 | sed -E 's/cursor: ([0-9.]+) pt/\1/'; }

# Whether two places are a point apart at most.
near() { awk "BEGIN { exit !($1 - $2 < 1 && $2 - $1 < 1) }"; }

# Presses keys after the reader has taken wtype's keyboard. wtype puts a
# space between separate words, so a sequence is one word: gg, not g g.
press() { wtype -s 800 "$@"; }

# How many lines of the log match `pattern`.
how_many() { grep -c -a -E "$1" said || true; }

# Presses keys that make `acts` acts, and waits until the reader has done
# them all.
keys=0
act() {
  local acts=$1
  shift
  press "$@"
  keys=$((keys + acts))
  logged '^key ' "$keys"
}

# Whether the selection shows `words` as the page sets them, said as Rust
# quotes them, by the log's last word on it, which comes before its key's,
# whichever kind of selection it is.
shows() {
  local said
  said=$(grep -a -E '^selected: ' said | tail -n 1)
  said=${said#selected: }
  said=${said#Chars }
  said=${said#Pieces }
  [ "$said" = "$1" ] || fail "the selection is $said, not $1"
}

# Waits for the next copy, and whether it took `markdown`, said as Rust
# quotes it.
copies=0
copied() {
  copies=$((copies + 1))
  logged '^copied: ' "$copies"
  local said
  said=$(grep -a -E '^copied: ' said | sed -n "${copies}p")
  [ "$said" = "copied: $1" ] || fail "copied ${said#copied: }, not $1"
}

deadline=$((SECONDS + 60))
until holds '^key Last None' 1; do
  ((SECONDS < deadline)) || fail "the reader never took G"
  press G
done
awk "BEGIN { exit !($(heading) > 1000) }" || fail "G went to $(heading) pt, not the end"

press gg
logged '^key First None: heading 0.0 pt' 1
# The top of the page, once its heading is drawn in the strong text.
deadline=$((SECONDS + 30))
until grim "$photo" \
  && [ "$(magick "$photo" -fill black +opaque "$strong" -fill white -opaque "$strong" \
    -format '%[fx:round(mean*w*h)]' info:)" -gt 100 ]; do
  ((SECONDS < deadline)) || fail "the page's heading never showed in the strong text"
  inotifywait -q -q -t 1 -e modify said || true
done
cp "$photo" at-rest.png

# How many pixels of a photo, left of the title's words, are `colour`.
left_of_words() {
  local box left
  box=$(magick "$1" -fill black +opaque "$strong" -fill white -opaque "$strong" \
    -format '%@' info:)
  left=${box#*+}
  left=${left%%+*}
  magick "$1" -crop "${left}x100000+0+0" +repage -fill black +opaque "$2" \
    -fill white -opaque "$2" -format '%[fx:round(mean*w*h)]' info:
}

# The cursor's line, at the top after gg, is marked by a rule in the
# accent in the margin left of the words, where nothing else is.
[ "$(left_of_words at-rest.png "$accent")" -gt 10 ] \
  || fail "the margin holds $(left_of_words at-rest.png "$accent") pixels of the accent, not a rule"

# Copying at rest, and selecting. The cursor is on the eyebrow over the
# title, so y y copies the block it's over, the title, and y i p its
# paragraph, the title and the lede after it with no blank line between,
# each as the file's markdown, onto the clipboard, and flashes it for
# 200 ms. y a p takes the blank line after it too, and y } the same
# paragraph to its end.
keys=$(how_many '^key ')
act 1 yy
copied '"# Every Key Neovim Has\n"'
logged '^flash: [1-9][0-9]* lines for 200 ms' 1
logged '^flash: over' 1
deadline=$((SECONDS + 20))
until [ "$(wl-paste -n 2> /dev/null)" = '# Every Key Neovim Has' ]; do
  ((SECONDS < deadline)) || fail "the clipboard holds $(wl-paste -n 2>&1), not the title"
  inotifywait -q -q -t 1 -e modify said || true
done
act 1 yip
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n"'
act 1 yap
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n\n"'
act 1 'y}'
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n"'
# j moves the cursor and not the view: two rows down, past the title, y y
# copies the lede alone, a block of its own, y i p still the paragraph
# round it, and y } only the lede, from it to the paragraph's end; then k
# and gg go back.
act 1 2j
act 1 yy
copied '"Some of it is on [the second page](second.md).\n"'
act 1 yip
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n"'
act 1 'y}'
copied '"Some of it is on [the second page](second.md).\n"'
near "$(heading)" 0 || fail "j moved the view to $(heading) pt"
act 1 k
act 1 yy
copied '"# Every Key Neovim Has\n"'
act 1 gg

# v starts on the first of the file's words in view, the title's, past
# the eyebrow over it, and } goes on by paragraphs, to the line of words
# after the title's. w goes on a word, iw takes it, and y copies it as the
# file has it, its markup and all, ending the selection.
act 1 v
shows '"E"'
# Its block is drawn as kitty draws one, in the cursor's colour with the
# letter in the cursor's text, though the title's letters are in the
# strong text rather than the page's: of what v changed on the page at
# rest, that many pixels are each of the two exactly.
changed_to() {
  magick v.png -fill black +opaque "$1" -fill white -opaque "$1" \
    \( at-rest.png v.png -compose difference -composite -colorspace gray -threshold 0 \) \
    -compose multiply -composite -format '%[fx:round(mean*w*h)]' info:
}
deadline=$((SECONDS + 30))
until grim v.png && [ "$(changed_to "$cursor")" -gt 100 ] \
  && [ "$(changed_to "$cursor_text")" -gt 20 ]; do
  ((SECONDS < deadline)) \
    || fail "v's cursor took $(changed_to "$cursor") pixels of its colour and $(changed_to \
      "$cursor_text") of its text's, not its block with the letter in it"
  inotifywait -q -q -t 1 -e modify said || true
done
act 1 -k Escape
shows 'nothing'
act 3 'v}w'
act 1 iw
shows '"bold"'
act 1 y
copied '"**bold**"'
shows 'nothing'
# Ending a selection leaves the cursor on its line, as Neovim's does, so
# v starts there again, and each selection after starts from gg.
act 1 v
shows '"A"'
act 1 -k Escape
# m while selecting marks the cursor's line and keeps the selection, as
# his Neovim's does, so ' and its letter at rest go to that line rather
# than the view's top, where the page still is.
act 5 'ggv}wiw'
act 1 ma
shows '"bold"'
act 1 -k Escape
act 1 "'a"
awk "BEGIN { exit !($(heading) > 0) }" || fail "'a went to $(heading) pt, not the marked line"
act 1 gg
# e takes the selection to a word's end, and Ctrl+C copies as y does.
act 5 'v}wiwe'
shows '"bold start"'
act 1 -M ctrl c -m ctrl
copied '"**bold** start"'
# $ goes to the line's end, b back by words, 0 to the line's start.
act 5 'ggv}$iw'
shows '"."'
act 1 b
shows '"sentence."'
act 1 b
shows '"second sentence."'
act 1 0
shows '"A bold start, then code in a line. A second sentence."'
act 1 y
copied '"A **bold** start, then `code` in a line. A second sentence."'
# o swaps the ends, so 0 moves the other; { goes back to the
# paragraph's start; Escape ends the selection.
act 7 'ggv}wiwo0'
shows '"A bold"'
act 1 -k Escape
shows 'nothing'
act 6 'ggv}wiw{'
shows '"A b"'
act 1 -k Escape
# A count goes with a motion; i` takes a code span's words, and a` its
# backticks and the space after them too, which the page doesn't show.
act 4 'ggv}5w'
act 1 'i`'
shows '"code"'
act 1 y
copied '"code"'
act 4 'ggv}5w'
act 1 'a`'
shows '"code "'
act 1 y
copied '"`code` "'
# is and as take a sentence, as without and with the space after it; aw
# a word with its space.
act 5 'ggv}wis'
shows '"A bold start, then code in a line."'
act 1 as
shows '"A bold start, then code in a line. "'
act 1 y
copied '"A **bold** start, then `code` in a line. "'
act 5 'ggv}waw'
shows '"bold "'
act 1 -k Escape
# The arrows extend as h, j, k and l do, the arrow first.
act 5 'ggv}wiw'
act 1 -k Right
shows '"bold "'
act 1 l
shows '"bold s"'
act 1 -k Left
shows '"bold "'
act 1 h
shows '"bold"'
act 1 -k Down
act 1 -k Up
shows '"bold"'
act 1 j
act 1 k
shows '"bold"'
act 1 -k Escape
# V takes whole blocks, here the title to the line of words, as the
# file's lines; on the code, a line of it; ip the code's paragraph, its
# fences with it, and ap the blank line after it too.
act 3 'ggV}'
act 1 y
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n\nA **bold** start, then `code` in a line. A second sentence.\n"'
act 6 'ggv}}iwV'
act 1 y
copied '"echo one\n"'
act 5 'ggV}}ip'
act 1 y
copied '"```sh\necho one\necho two\n```\n"'
act 5 'ggv}}ap'
act 1 y
copied '"```sh\necho one\necho two\n```\n\n"'
# v again ends it, and V makes it a selection of blocks.
act 2 vv
shows 'nothing'
act 2 vV
[[ "$(grep -a -E '^selected: ' said | tail -n 1)" == 'selected: Pieces '* ]] \
  || fail "V didn't make the selection one of blocks"
act 1 -k Escape
shows 'nothing'
# A paragraph the page wraps is one line of the file, and is copied as
# one: $ stops where the page broke it, and 22e goes past the break to
# its last word. Five } pass the title's paragraph, the one after it,
# two code blocks and Part 1's heading.
act 8 'ggv}}}}}iw'
shows '"Paragraph"'
act 1 '$'
said=$(grep -a -E '^selected: ' said | tail -n 1)
[[ $said != *somewhere* ]] || fail "the paragraph fits a line, so nothing wraps: $said"
act 1 -k Escape
act 9 'ggv}}}}}iw22e'
whole='Paragraph 1 says enough to take a line or two of the page, so that every key here has somewhere to go'
shows "\"$whole\""
act 1 y
copied "\"$whole\""
deadline=$((SECONDS + 10))
until [ "$(wl-paste -n 2> /dev/null)" = "$whole" ]; do
  ((SECONDS < deadline)) || fail "the clipboard holds $(wl-paste -n 2>&1), not the paragraph"
  sleep 0.1
done

press 5j
logged '^key Down Some\(5\)' 1

press -M ctrl f -m ctrl
logged '^key ScreenDown None' 1
awk "BEGIN { exit !($(heading) > 100) }" || fail "Ctrl+F stayed at $(heading) pt"

press -M ctrl o -m ctrl
logged '^key JumpBack None' 1

press -M ctrl = -m ctrl
logged '^key ZoomIn None' 1
press -M ctrl 0 -m ctrl
logged '^key ZoomBack None' 1

press /Every
press -k Return
logged '^search ' 1
grep -a -q -E '^search .*[1-9][0-9]* found' said || fail "/ found nothing"

press n
logged '^key Next None' 1

press ' ?'
logged '^key ShowKeys None' 1
rests=$(how_many '^key Rest None')
press -k Escape
logged '^key Rest None' $((rests + 1))

# The link at the page's end, back to its heading, by its label.
ends=$(grep -c -a '^key Last None' said || true)
press G
logged '^key Last None' $((ends + 1))
press gx
logged '^links: 1 in view' 1
press a
logged '^link: #every-key-neovim-has' 1
downs=$(how_many '^key Down None')
press j
logged '^key Down None' $((downs + 1))
awk "BEGIN { exit !($(heading) < 100) }" || fail "the link went to $(heading) pt, not the top"
awk "BEGIN { exit !($(cursor) < 100) }" || fail "the link left the cursor at $(cursor) pt"

# The link near the top to another markdown file sets it in this window,
# and Ctrl+O comes back to the place it left on the first, Ctrl+I goes on
# to it again, and the file shown and its saves move with it. Ctrl+E
# takes the view a line down, leaving a place to come back to while the
# link, the page's first line under its heading, stays in view on any
# line the page is set on, and Ctrl+Y back up and down again prove they
# move the view alone, a line each.
linked=$(heading)
press -M ctrl e -m ctrl
logged '^key ScrollDown None' 1
lower=$(heading)
awk "BEGIN { exit !($lower > $linked) }" || fail "Ctrl+E left the view at $lower pt"
kept=$(cursor)
press -M ctrl y -m ctrl
logged '^key ScrollUp None' 1
near "$(heading)" "$linked" || fail "Ctrl+Y took the view to $(heading) pt, not $linked"
near "$(cursor)" "$kept" || fail "Ctrl+Y moved the cursor from $kept pt to $(cursor)"
press -M ctrl e -m ctrl
logged '^key ScrollDown None' 2
left=$(heading)
press gx
logged '^links: 1 in view' 2
press a
logged '^link: second.md' 1
logged '^opened: (.*/)?second.md, heading 0.0 pt' 1
grep -q 'second.md$' state/reader/showing/[0-9]* \
  || fail "the file shown is said as $(cat state/reader/showing/[0-9]*)"
press -M ctrl o -m ctrl
logged '^opened: (.*/)?page.md, ' 1
back=$(grep -a -E '^opened: (.*/)?page.md, ' said | sed -E 's/.*heading ([0-9.]+) pt/\1/')
awk "BEGIN { exit !($back - $left < 1 && $left - $back < 1) }" \
  || fail "Ctrl+O came back to $back pt, not $left"
press -M ctrl i -m ctrl
logged '^opened: (.*/)?second.md, ' 2
echo 'Written on since.' >> second.md
logged '^saved: ' 1

# How many lines the log holds matching `pattern`.
seen() { grep -c -a -E "$1" said || true; }

# [b goes back to the markdown file before this one in the folder, and ]b
# on to the next, round its ends.
press '[b'
logged '^file: (.*/)?page.md' 1
logged '^opened: (.*/)?page.md, ' 2
press ']b'
logged '^file: (.*/)?second.md' 1
logged '^opened: (.*/)?second.md, ' 3
press '[b'
logged '^opened: (.*/)?page.md, ' 3

# } and { by paragraphs, ]] and [[ by headings, from the top. A selection
# took them already, so they're counted on from what it left.
tops=$(seen '^key First None')
ons=$(seen '^key ParagraphOn None')
backs=$(seen '^key ParagraphBack None')
press gg
logged '^key First None' $((tops + 1))
top=$(heading)
press '}'
logged '^key ParagraphOn None' $((ons + 1))
paragraph=$(heading)
awk "BEGIN { exit !($paragraph > $top) }" || fail "} stayed at $paragraph pt"
press '{'
logged '^key ParagraphBack None' $((backs + 1))
awk "BEGIN { exit !($(heading) < $paragraph) }" || fail "{ stayed at $(heading) pt"
press ']]'
logged '^key HeadingOn None' 1
part=$(heading)
awk "BEGIN { exit !($part > $paragraph) }" || fail "]] went to $part pt"
press '2]]'
logged '^key HeadingOn Some\(2\)' 1
awk "BEGIN { exit !($(heading) > $part) }" || fail "2]] went to $(heading) pt"
press '[['
logged '^key HeadingBack None' 1

# H, M and L put the cursor on the view's top, middle and bottom lines,
# and zt puts the bottom one at the top; zz and zb place it too.
press H
logged '^key ViewTop None' 1
high=$(cursor)
press M
logged '^key ViewMiddle None' 1
middle=$(cursor)
press L
logged '^key ViewBottom None' 1
low=$(cursor)
awk "BEGIN { exit !($high < $middle && $middle < $low) }" \
  || fail "H, M and L put the cursor at $high, $middle and $low pt"
press zt
logged '^key PlaceTop None' 1
near "$(heading)" "$low" || fail "zt put the view at $(heading) pt, not $low"
press zz
logged '^key PlaceMiddle None' 1
press zb
logged '^key PlaceBottom None' 1

# 50% goes half way down, and % from the code block's { to its }: the
# title and the line under it are one paragraph, so it's the third on.
press 50%
logged '^key Bracket Some\(50\)' 1
press gg
logged '^key First None' $((tops + 2))
press '3}'
logged '^key ParagraphOn Some\(3\)' 1
open=$(cursor)
press %
logged '^key Bracket None' 1
awk "BEGIN { exit !($(cursor) > $open) }" || fail "% left the cursor at $(cursor) pt"

# * and # search for the cursor's line's first word, whole.
press ']]}'
logged '^key ParagraphOn None' $((ons + 2))
press '*'
logged '^search SameWordOn ' 1
grep -a -q -F '"\\<Paragraph\\>", true' said || fail "* searched for something else"
grep -a -q -E '^search SameWordOn .* 120 found' said || fail "* found other than 120"
press '#'
logged '^search SameWordBack ' 1

# The command line: :noh, a line's number, and anything else is E492.
press ':noh'
press -k Return
logged '^command: "noh" is NoHighlight' 1
press ':5'
press -k Return
logged '^command: "5" is Line\(5\)' 1
press ':foo'
press -k Return
logged '^said: E492: Not an editor command: foo' 1

# gO lists the five headings, j chooses the next, Return goes to it.
press gg
logged '^key First None' $((tops + 3))
press gO
logged '^outline: 5 rows, chosen "Every Key Neovim Has"' 1
press j
logged '^outline: 5 rows, chosen "Part 1"' 1
press -k Return
logged '^outline went to "Part 1"' 1
press gO
logged '^outline: 5 rows, chosen "Part 1"' 2
press -k Escape
logged '^outline: closed' 2

# A mark is kept with its file between openings.
press ma
logged '^key SetMark\(.a.\)' 1
marked=$(heading)
press ']b'
logged '^opened: (.*/)?second.md, ' 4
press '[b'
logged '^opened: (.*/)?page.md, ' 4
press gg
logged '^key First None' $((tops + 4))
press "'a"
logged '^key GoToMark\(.a.\)' 1
near "$(heading)" "$marked" || fail "'a went to $(heading) pt, not $marked"

# gf opens the file the top's link names, and Ctrl+] follows it.
press gg
logged '^key First None' $((tops + 5))
press gf
logged '^file: (.*/)?second.md' 3
logged '^opened: (.*/)?second.md, ' 5
press -M ctrl o -m ctrl
logged '^opened: (.*/)?page.md, ' 5
press -M ctrl ']' -m ctrl
logged '^link: second.md' 2
logged '^opened: (.*/)?second.md, ' 6

# :q closes it.
press ':q'
press -k Return
wait "$reader" || true
grep -a -q -E '^command: "q" is Quit' said || fail ":q didn't close it"
touch passed
