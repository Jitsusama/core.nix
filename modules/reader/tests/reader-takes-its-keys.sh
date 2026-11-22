# shellcheck shell=bash
# Runs inside cage: opens a long markdown file in the reader, presses
# Neovim's keys through wtype and reads where each took the view from the
# reader's key log (READER_KEYS_LOG), shoots the top of the page once
# its heading is drawn, and closes it with :q. cage doesn't pass on how this
# ended, so it leaves ./passed once every key did what it should.
#
# Long because it's one reader in one cage, driven key by key, and each
# case starts where the one before left the view and the cursor: split,
# each part would start cage and the reader again and replay the keys
# that brought it there.
#
# cage has no keyboard until wtype makes one, so the reader has the
# keyboard only once the first key is typed: G is pressed until it's taken.
# Each wtype adds a keyboard of its own and takes it away as it ends, so
# each one waits before typing for the reader to take the new keyboard.
#
# READER_LATE_FENCE has each texture's first look find the GPU still
# filling it, as a busy machine's does, so the reader has to look again
# by itself: nothing else comes to wake a window that hasn't shown yet.
#
# usage: reader-takes-its-keys.sh <presses.sh> <file> <photo> <heading colour> <cursor> <cursor text> <accent>
#
# The markdown a copy is held to holds backticks and dollars, meant as
# themselves, so it's quoted whole in single quotes.
# shellcheck disable=SC2016
set -euo pipefail

presses=$1
file=$2
photo=$3
strong=$4
cursor=$5
cursor_text=$6
accent=$7
: > said

READER_KEYS_LOG=1 READER_LATE_FENCE=1 reader "$file" > said 2>&1 &

# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/presses.sh
source "$presses" "$!" "$strong"

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
# copies the lede alone, a block of its own, y } only the lede, from it
# to the paragraph's end, and y i p the paragraph round it, which takes
# the cursor to the paragraph's start, the title, as Vim's does; then k
# and gg go back.
act 1 2j
act 1 yy
copied '"Some of it is on [the second page](second.md).\n"'
act 1 'y}'
copied '"Some of it is on [the second page](second.md).\n"'
lede=$(cursor)
act 1 yip
copied '"# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n"'
awk "BEGIN { exit !($(cursor) < $lede - 10) }" \
  || fail "y i p left the cursor at $(cursor) pt, not the paragraph's start"
near "$(heading)" 0 || fail "j moved the view to $(heading) pt"
act 1 k
act 1 yy
copied '"# Every Key Neovim Has\n"'
act 1 gg

# The keyboard's repeats move and do nothing else: y and a held y copy
# once, from the press, and a held j goes on down. g g after them is how
# the test knows the repeats are all in. After a held key the reader can
# read the next wtype's first keys by the keymap of the one before, whose
# first key is wtype's Escape here, so a key misread does nothing, and g g
# goes again until it's read as itself.
flashes=$(how_many '^flash: [1-9]')
downs=$(how_many '^key Down ')
firsts=$(how_many '^key First ')
press -k Escape y -P y -s 1500 -p y -P j -s 1500 -p j
deadline=$((SECONDS + 30))
until [ "$(how_many '^key First ')" -gt "$firsts" ]; do
  ((SECONDS < deadline)) || fail "the reader never took g g after the held keys"
  press gg
  inotifywait -q -q -t 2 -e modify said || true
done
keys=$(how_many '^key ')
copied '"# Every Key Neovim Has\n"'
[ "$(how_many '^copied: ')" -eq "$copies" ] \
  || fail "a held y copied $(($(how_many '^copied: ') - copies + 1)) times, not once"
[ "$(how_many '^flash: [1-9]')" -eq $((flashes + 1)) ] \
  || fail "a held y flashed $(($(how_many '^flash: [1-9]') - flashes)) times, not once"
[ "$(how_many '^key Down ')" -gt $((downs + 2)) ] \
  || fail "a held j moved $(($(how_many '^key Down ') - downs)) times, not on and on"

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
  local shot=${2:-v.png}
  magick "$shot" -fill black +opaque "$1" -fill white -opaque "$1" \
    \( at-rest.png "$shot" -compose difference -composite -colorspace gray -threshold 0 \) \
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
# Ending it leaves the cursor on the selection's cursor, as Vim's does.
stands '"E"'
act 3 'v}w'
act 1 iw
shows '"bold"'
act 1 y
copied '"**bold**"'
shows 'nothing'
# y leaves the cursor on the start of what it took, as Vim's does, and
# Escape at rest then puts the cursor back to being its line, so v starts
# on its first word again; each selection after starts from gg.
stands '"b"'
act 1 -k Escape
stands 'nothing'
act 1 v
shows '"A"'
act 1 -k Escape
act 1 -k Escape
# At rest, the keys along a line put the cursor on a character, from the
# first word on its line, drawn as v's block is, and v starts there: w to
# bold's b, e to its end, then v and b select it back to its start.
act 1 w
stands '"b"'
deadline=$((SECONDS + 30))
while :; do
  took=0
  if grim w.png; then took=$(changed_to "$cursor" w.png); fi
  ((took > 100)) && break
  ((SECONDS < deadline)) || fail "w's cursor took $took pixels of its colour, not its block"
  inotifywait -q -q -t 1 -e modify said || true
done
act 1 e
stands '"d"'
act 1 v
shows '"d"'
stands 'nothing'
act 1 b
shows '"bold"'
act 1 y
copied '"**bold**"'
# f goes to the character typed, and ; to it again, so v e then takes
# the rest of second's word.
act 1 fc
stands '"c"'
act 1 ';'
act 2 ve
shows '"cond"'
act 1 -k Escape
stands '"d"'
act 1 -k Escape
# j and k carry the character, its line with it, keeping across, so it
# comes back to bold's b; Escape puts the cursor back to its line, so v
# starts on its first word.
act 1 w
stands '"b"'
before=$(cursor)
act 1 j
after=$(cursor)
awk "BEGIN { exit !($after > $before) }" || fail "j left the cursor at $after pt"
act 1 k
stands '"b"'
after=$(cursor)
awk "BEGIN { exit !($after - $before < 1 && $before - $after < 1) }" \
  || fail "k put the cursor at $after pt, not $before"
act 1 -k Escape
stands 'nothing'
# And its block goes with it: nothing on the page is in the cursor's
# colour that wasn't at rest.
deadline=$((SECONDS + 30))
while :; do
  left=999
  if grim gone.png; then left=$(changed_to "$cursor" gone.png); fi
  ((left < 20)) && break
  ((SECONDS < deadline)) || fail "Escape left $left pixels of the cursor's block"
  inotifywait -q -q -t 1 -e modify said || true
done
act 1 v
shows '"A"'
act 1 -k Escape
stands '"A"'
# A search let go leaves the cursor where it stood, so l goes on from
# there; F goes back to a character and , on to the next of it, the
# other way, as Vim's do, so v e takes the second sentence's start.
cancels=$(how_many '^search Cancelled ')
act 1 /zzz
press -k Escape
logged '^search Cancelled ' $((cancels + 1))
act 1 l
stands '" "'
act 1 FA
stands '"A"'
act 1 ,
act 2 ve
shows '"A second"'
act 1 -k Escape
act 1 -k Escape
stands 'nothing'
# A jump from the command line takes the cursor off the character's
# line, and with it the character, so w goes on from the line it's on.
press ':1'
press -k Return
logged '^command: "1" is Go' 1
keys=$(how_many '^key ')
act 1 w
stands '"K"'
act 1 -k Escape
stands 'nothing'
# m while selecting marks the cursor's line and keeps the selection, as
# his Neovim's does, so ' and its letter at rest put the cursor back on
# that line, and the view stays, since the line shows.
act 5 'ggv}wiw'
act 1 ma
shows '"bold"'
act 1 -k Escape
marked=$(cursor)
act 1 gg
awk "BEGIN { exit !($(cursor) < $marked) }" || fail "gg left the cursor at $(cursor) pt"
act 1 "'a"
near "$(cursor)" "$marked" || fail "'a put the cursor at $(cursor) pt, not $marked"
near "$(heading)" 0 || fail "'a moved the view to $(heading) pt, though the line showed"
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
# y leaves the cursor at the selection's start, as Vim's does, so y y
# copies the title rather than the line the selection ended on.
act 1 yy
copied '"# Every Key Neovim Has\n"'
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

# v after a search starts at its match, as Neovim's starts at the cursor's
# character, which the search put there: here the page's one sentence,
# halfway along its line.
searches=$(how_many '^search Done ')
press /sentence
press -k Return
logged '^search Done ' $((searches + 1))
selected=$(how_many '^selected: ')
press v
logged '^selected: ' $((selected + 1))
shows '"s"'
press -k Escape
logged '^selected: ' $((selected + 2))
shows 'nothing'

press ' ?'
logged '^key ShowKeys None' 1
rests=$(how_many '^key Rest None')
aways=$(how_many '^keys: away')
press -k Escape
logged '^key Rest None' $((rests + 1))
# Escape closes nothing, the key list included; ZZ closes the list alone,
# and the reader stays for the keys after it.
[ "$(how_many '^keys: away')" = "$aways" ] || fail "Escape put the key list away"
closes=$(how_many '^key Close None')
press ZZ
logged '^key Close None' $((closes + 1))
logged '^keys: away' $((aways + 1))

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
tops=$(how_many '^key First None')
ons=$(how_many '^key ParagraphOn None')
backs=$(how_many '^key ParagraphBack None')
# Each puts the cursor on the line it goes to, as Vim's: the view stays
# while the line shows, and a heading a screen on is set in its middle.
press gg
logged '^key First None' $((tops + 1))
top=$(cursor)
press '}'
logged '^key ParagraphOn None' $((ons + 1))
paragraph=$(cursor)
awk "BEGIN { exit !($paragraph > $top) }" || fail "} left the cursor at $paragraph pt"
near "$(heading)" 0 || fail "} moved the view to $(heading) pt, though its line showed"
press '{'
logged '^key ParagraphBack None' $((backs + 1))
awk "BEGIN { exit !($(cursor) < $paragraph) }" || fail "{ left the cursor at $(cursor) pt"
press ']]'
logged '^key HeadingOn None' 1
part=$(cursor)
awk "BEGIN { exit !($part > $paragraph) }" || fail "]] put the cursor at $part pt"
near "$(heading)" 0 || fail "]] moved the view to $(heading) pt, though its heading showed"
press '2]]'
logged '^key HeadingOn Some\(2\)' 1
awk "BEGIN { exit !($(cursor) > $part && $(heading) > 0 && $(heading) < $(cursor)) }" \
  || fail "2]] put the cursor at $(cursor) pt and the view at $(heading) pt"
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
logged '^command: "5" is Go\(Lines\(5, 5\)\)' 1
press ':foo'
press -k Return
logged '^said: E492: Not an editor command: foo' 1
press ':%q'
press -k Return
logged '^said: E481: No range allowed' 1

# The line's keys are the key language's: Ctrl+W takes the last word,
# Ctrl+U the rest and Ctrl+H a letter, and Ctrl+C drops the line, so the
# :5 after it is a line of its own.
press ':abc def'
press -M ctrl w -m ctrl
press -M ctrl u -m ctrl
press nohx
press -M ctrl h -m ctrl
press -k Return
logged '^command: "noh" is NoHighlight' 2
press ':q'
press -M ctrl c -m ctrl
press ':5'
press -k Return
logged '^command: "5" is Go\(Lines\(5, 5\)\)' 2

# :%y copies the file as it's written and says how many lines, as Vim
# does; :'<,'>y the lines the last selection took, once it has ended;
# and a range past the file's end is E16.
taken=$(how_many '^copied: ')
press ':%y'
press -k Return
logged '^copied: ' $((taken + 1))
logged "^said: $(wc -l < page.md) lines yanked\$" 1
deadline=$((SECONDS + 10))
until [ "$(wl-paste 2> /dev/null)" = "$(cat page.md)" ]; do
  ((SECONDS < deadline)) || fail "the clipboard holds $(wl-paste 2>&1 | head -n 1), not the file"
  inotifywait -q -q -t 1 -e modify said || true
done
# :1 rather than gg, so the First keys counted on below stay as many.
press ':1'
press -k Return
logged '^command: "1" is Go\(Lines\(1, 1\)\)' 2
lines=$(how_many '^key SelectPieces None')
press V
logged '^key SelectPieces None' $((lines + 1))
downs=$(how_many '^key Down None')
press j
logged '^key Down None' $((downs + 1))
ended=$(how_many '^selected: nothing')
press -k Escape
logged '^selected: nothing' $((ended + 1))
press ":'<,'>y"
press -k Return
logged '^copied: ' $((taken + 2))
[ "$(grep -a -E '^copied: ' said | tail -n 1)" \
  = 'copied: "# Every Key Neovim Has\nSome of it is on [the second page](second.md).\n"' ] \
  || fail ":'<,'>y copied $(grep -a -E '^copied: ' said | tail -n 1)"
press ':1,99999y'
press -k Return
logged '^said: E16: Invalid range' 1

# Escape at rest does nothing, as Vim's does: the search's marks stay, n
# having put them back after :noh.
nexts=$(how_many '^key Next None')
marks=$(how_many '^search Rest .* marked true')
press n
logged '^key Next None' $((nexts + 1))
press -k Escape
logged '^search Rest .* marked true' $((marks + 1))

# Every change is refused with Vim's E21, whole: d and c once their motion
# is, a mark's letter and a text object among them, :d, and d on a
# selection, which Escape then ends.
refused=0
for change in dd cc S x p dj cw dip "d'a" c3j; do
  press "$change"
  refused=$((refused + 1))
  logged '^said: E21: Cannot make changes$' "$refused"
done
press ':d'
press -k Return
logged '^said: E21: Cannot make changes$' $((refused + 1))
selects=$(how_many '^key Select None')
press v
logged '^key Select None' $((selects + 1))
press d
logged '^said: E21: Cannot make changes$' $((refused + 2))
rests=$(how_many '^key Rest None')
press -k Escape
logged '^key Rest None' $((rests + 1))

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
# Escape closes nothing, so j still moves through the outline, and ZZ
# closes it as :q would a window.
press -k Escape
press j
logged '^outline: 5 rows, chosen "Part 2"' 1
[ "$(how_many '^outline: closed')" = 1 ] || fail "Escape closed the outline"
press ZZ
logged '^outline: closed' 2

# A mark is kept with its file between openings.
press ma
logged '^key SetMark\(.a.\)' 1
marked=$(cursor)
press ']b'
logged '^opened: (.*/)?second.md, ' 4
press '[b'
logged '^opened: (.*/)?page.md, ' 4
press gg
logged '^key First None' $((tops + 4))
press "'a"
logged '^key GoToMark\(.a.\)' 1
near "$(cursor)" "$marked" || fail "'a put the cursor at $(cursor) pt, not $marked"

# A mark on a heading's own line, ]] from the top, and one on the last
# line, by G, are kept as :q closes it, and read back once it has.
press gg
logged '^key First None' $((tops + 5))
press ']]'
logged '^key HeadingOn None' 2
press mh
logged '^key SetMark\(.h.\)' 1
lasts=$(how_many '^key Last None')
press G
logged '^key Last None' $((lasts + 1))
press mz
logged '^key SetMark\(.z.\)' 1

# Selecting, gf ends the selection and takes the words it held on one
# line as the name, as Vim's v_gf does, here the lede's, which no file
# has, never the link they hold.
press gg
logged '^key First None' $((tops + 6))
press '2jv$'
ended=$(how_many '^selected: nothing')
e447=$(how_many '^said: E447')
press gf
logged '^selected: nothing' $((ended + 1))
logged '^said: E447: Can.t find file "Some of it is on .*" in path' $((e447 + 1))
# Over two lines, as Vim's, it names none, even with its cursor on the
# lede's start, where the link after it would be gf's without a selection.
press 0
press jVk
ended=$(how_many '^selected: nothing')
e446=$(how_many '^said: E446')
press gf
logged '^selected: nothing' $((ended + 1))
logged '^said: E446: No file name under cursor' $((e446 + 1))

# gf opens the file the top's link names, and Ctrl+] follows it.
press gg
logged '^key First None' $((tops + 7))
press gf
logged '^file: (.*/)?second.md' 3
logged '^opened: (.*/)?second.md, ' 5
press -M ctrl o -m ctrl
logged '^opened: (.*/)?page.md, ' 5
press -M ctrl ']' -m ctrl
logged '^link: second.md' 2
logged '^opened: (.*/)?second.md, ' 6

# :bp and :bn go back and on through the folder as [b and ]b do, :e
# opens a file named from where the reader started and says E484 of one
# that isn't there, and :h lists the keys, which Space w puts away and
# Escape doesn't.
press ':bp'
press -k Return
logged '^opened: (.*/)?page.md, ' 6
press ':e second.md'
press -k Return
logged '^opened: (.*/)?second.md, ' 7
press ':e nowhere.md'
press -k Return
logged '^said: E484: Can.t open file nowhere.md' 1
press ':bn'
press -k Return
logged '^opened: (.*/)?page.md, ' 7
press ':h'
press -k Return
logged '^command: "h" is Help' 1
rests=$(how_many '^key Rest None')
aways=$(how_many '^keys: away')
press -k Escape
logged '^key Rest None' $((rests + 1))
[ "$(how_many '^keys: away')" = "$aways" ] || fail "Escape put :h's list away"
press ' w'
logged '^keys: away' $((aways + 1))

# :g/Part/ lists the four sections' headings, the lines it matches, in
# gO's place, and Return goes to the one chosen; one matching nothing
# says so. :marks lists ', a, h and z, and :ju the jumps, each put away
# by ZZ or Space w.
closed=$(how_many '^outline: closed')
went=$(how_many '^outline went to')
press ':g/Part/'
press -k Return
logged '^outline: 4 rows, chosen "Part [1-4]"' 1
press -k Return
logged '^outline went to "Part [1-4]"' $((went + 1))
press ':g/zzz/'
press -k Return
logged '^said: Pattern not found: zzz' 1
press ':marks'
press -k Return
logged "^outline: 4 rows, chosen \"'  " 1
press ZZ
logged '^outline: closed' $((closed + 2))
lists=$(how_many '^outline: [0-9]+ rows')
press ':ju'
press -k Return
logged '^command: "ju" is Jumps' 1
logged '^outline: [0-9]+ rows' $((lists + 1))
press ' w'
logged '^outline: closed' $((closed + 3))

# A row too long for the list reads on while it's chosen: its words
# slide to show their end after a rest, as the launcher's long names do.
press ':g/Paragraph 7 says/'
press -k Return
logged '^outline: 1 rows, chosen "Paragraph 7 says' 1
logged '^outline: reads on [0-9]+ px past its room' 1
logged '^outline: reading on' 1
logged '^outline: read on to its end' 1
press ZZ
logged '^outline: closed' $((closed + 4))

# The menu shows the moment : is typed, every command the page has; a
# letter narrows it, Tab, Shift+Tab, the arrows, Ctrl+N and Ctrl+P choose
# a line and come round through what was typed, Ctrl+E puts it away with
# what was typed back, and Ctrl+Y takes the line chosen. After :e and a
# space, it lists the files, and Return runs what the line holds.
# Waits until the menu's last line logged says `menu`, as Rust quotes it.
menu_is() {
  local deadline=$((SECONDS + 50))
  until [ "$(grep -a -E '^menu: ' said | tail -n 1)" = "menu: $1" ]; do
    ((SECONDS < deadline)) || fail "the $(grep -a -E '^menu: ' said | tail -n 1), not $1"
    inotifywait -q -q -t 1 -e modify said || true
  done
}
press ':'
menu_is '12 lines, chosen None'
# e starts :e's name, then a word of List Earlier Lines, :his's.
press e
menu_is '2 lines, chosen None'
press -k Tab
menu_is '2 lines, chosen Some("e[dit]")'
press -M ctrl e -m ctrl
press -k Return
logged '^command: "e" is Edit\(None\)' 1
opens=$(how_many '^opened: (.*/)?second.md, ')
press ':e sec'
press -k Tab
menu_is '1 lines, chosen Some("second.md")'
press -k Return
logged '^command: "e second.md" is Edit' 1
logged '^opened: (.*/)?second.md, ' $((opens + 1))
press ':b'
menu_is '2 lines, chosen None'
press -k Tab
menu_is '2 lines, chosen Some("bn[ext]")'
press -k Down
menu_is '2 lines, chosen Some("bp[revious]")'
press -M shift -k Tab -m shift
menu_is '2 lines, chosen Some("bn[ext]")'
press -M ctrl p -m ctrl
menu_is '2 lines, chosen None'
press -k Up
menu_is '2 lines, chosen Some("bp[revious]")'
press -M ctrl n -m ctrl
menu_is '2 lines, chosen None'
press -M ctrl n -m ctrl
menu_is '2 lines, chosen Some("bn[ext]")'
press -M ctrl y -m ctrl
menu_is '1 lines, chosen None'
press -k Return
logged '^command: "bnext" is File' 1

# @: runs the last line again, and :his lists the lines run, the newest,
# itself, chosen; k and Return run the one before it, bnext.
press '@:'
logged '^command: "bnext" is File' 2
press ':his'
press -k Return
logged '^outline: [0-9]+ rows, chosen "[0-9]+  his"' 1
press k
press -k Return
logged '^command: "bnext" is File' 3

# :q closes it.
press ':q'
press -k Return
wait "$reader" || true
grep -a -q -E '^command: "q" is Quit' said || fail ":q didn't close it"

# The mark on the heading's line is under that heading, though its line's
# top is above where the heading's block starts, and the one on the last
# line says its line's share of the whole page, short of all of it, where
# a view's share there would be the whole.
kept=$(reader --places | grep -F '/page.md"') || fail "no place kept for page.md"
marked() { grep -o -E "\\{[^{}]*\"letter\":\"$1\"[^{}]*\\}" <<< "$kept" || true; }
on_heading=$(marked h)
on_last=$(marked z)
[[ $on_heading == *'"heading":"Part 1"'* ]] || fail "mark h on Part 1 was kept as $on_heading"
[[ $on_last =~ \"through\":([0-9.e-]+) ]] || fail "mark z said no share: $on_last"
awk "BEGIN { exit !(${BASH_REMATCH[1]} > 0.5 && ${BASH_REMATCH[1]} < 1) }" \
  || fail "mark z on the last line: $on_last"
touch passed
