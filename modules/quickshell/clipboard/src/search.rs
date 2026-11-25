//! Emoji's search, ranked here rather than by the shell (Joel, 2026-11-10:
//! "the daemons need to be the data layer, and they need to serve the
//! launcher"), so the shell makes a row only for an emoji it shows, where
//! it made and matched every one of 1,914 at its start. The `search` ask
//! names the place and what's typed, and is answered with the keys of the
//! emoji it finds, best first, and the letters of each name to bring
//! forward, as ready-rows.md says; `within` a group's row, only among
//! those inside it.
//!
//! An emoji is found by what it's called or what it means, as the shell's
//! Emoji.js found one before: each word typed has to start a word of its
//! name ("thumbs" finds "thumbs up") or of one of its keywords ("tada"
//! finds the party popper), and failing both, the name is matched loosely
//! as everything else is (`ready_rows::fuzzy`), for letters typed out of
//! place. A name counts for more than a keyword, since it's what the emoji
//! is, and a whole word for more than the start of one. Among names found
//! as well, the one with fewest words left over ("cat" before "grinning
//! cat"); then Unicode's own order, which puts the faces people use first
//! ("grinning face" before "grinning cat"). Keywords are matched only by
//! where their words start, not loosely, since they're long and many.

use ready_rows::fuzzy::{self, Item, Typed, runs};
use serde_json::{Map, Value, json};

use crate::emoji::PLACE;

/// What a word typed is worth where it whole is a word of the name, where
/// it starts one, where it's a keyword and where it starts one.
const NAME: i32 = 40;
const NAME_START: i32 = 30;
const KEYWORD: i32 = 20;
const KEYWORD_START: i32 = 10;
/// What the words' worth is multiplied by, so the least of it outweighs
/// any number of a name's words left over, and a score from it is never
/// below a loose match's.
const WORTH_SCALE: f64 = 10_000.0;
/// What each word of a name left over takes from a score.
const LEFT_OVER: f64 = 100.0;
/// What a loose match's score is divided by, to rank it below the rest.
const LOOSE_SCALE: f64 = 1000.0;
/// The most emoji an answer holds where the ask doesn't say, more than a
/// launcher shows at once.
const FOUND: usize = 100;
/// The most of what's typed a search reads, far past any emoji's name.
const LONGEST: usize = 200;

/// One emoji as a search reads it, worked out once as the history starts.
struct Emoji {
    key: String,
    /// The key of its group's row.
    parent: String,
    /// The words of its name and of its keywords, each with a space before
    /// and after, so " " and a word typed finds where one starts, and a
    /// space after it too finds a whole word.
    named: String,
    meant: String,
    /// How many words its name has.
    words: u32,
    /// Its name as its row draws it, matched loosely, and marked.
    name: Item,
}

/// Every emoji the history serves, ready to be searched.
pub struct Index {
    emoji: Vec<Emoji>,
}

impl Index {
    /// The index of the emoji among the place's records, in their order.
    pub fn of(records: &[Value]) -> Index {
        let emoji = records
            .iter()
            .map(|record| &record["row"])
            .filter(|row| row["kind"] == "emoji")
            .map(|row| {
                let name = text(&row["name"]);
                let keywords = row["keywords"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ");
                let named = spaced(name);
                let words = u32::try_from(named.trim().split(' ').count()).unwrap_or(u32::MAX);
                Emoji {
                    key: text(&row["key"]).to_owned(),
                    parent: text(&row["parent"]).to_owned(),
                    named,
                    meant: spaced(&keywords),
                    words,
                    name: Item::of(name),
                }
            })
            .collect();
        Index { emoji }
    }

    /// What a search the shell asks is answered with: the keys of what it
    /// finds, best first, at most as many as it says or `FOUND`, the
    /// letters of each name to bring forward, how many it found in all, so
    /// the launcher's head says those past the keys ("100 of 838"), and its
    /// grade, 1 where the best was found only loosely, as Fuzzy.js grades a
    /// match. A search of another place is refused, so the shell matches its
    /// rows itself.
    pub fn answer(&self, said: &Value) -> Value {
        let (place, words) = (text(&said["place"]), text(&said["words"]));
        if place != PLACE {
            return json!({
                "words": words,
                "error": {"code": "unknown-place", "message": format!("no place {place:?} here")},
            });
        }
        let most = said["most"]
            .as_u64()
            .and_then(|most| usize::try_from(most).ok())
            .unwrap_or(FOUND);
        // Only so much of what's typed is read, so no peer on the socket
        // holds the history matching a long text against every emoji.
        let read: String = words.chars().take(LONGEST).collect();
        let typed = Typed::of(&read);
        let found = self.found(&read, &typed, said["within"].as_str());
        let grade = u8::from(found.first().is_some_and(|(score, _)| *score < WORTH_SCALE));
        let all = found.len();
        let mut keys = Vec::new();
        let mut marks = Map::new();
        for (_, emoji) in found.into_iter().take(most) {
            if let Some((_, marked)) = emoji.name.marked(&typed) {
                marks.insert(emoji.key.clone(), json!(runs(&marked)));
            }
            keys.push(emoji.key.as_str());
        }
        json!({"words": words, "keys": keys, "marks": marks, "found": all, "grade": grade})
    }

    /// Every emoji what's typed finds, best first, those alike in score in
    /// Unicode's order; within a group's row, only those inside it.
    fn found(&self, words: &str, typed: &Typed, within: Option<&str>) -> Vec<(f64, &Emoji)> {
        let asked: Vec<(String, String)> = fuzzy::words(words)
            .iter()
            .map(|word| (format!(" {word}"), format!(" {word} ")))
            .collect();
        if asked.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(f64, &Emoji)> = self
            .emoji
            .iter()
            .filter(|emoji| within.is_none_or(|within| emoji.parent == within))
            .filter_map(|emoji| Some((score(emoji, &asked, typed)?, emoji)))
            .collect();
        // A stable sort, so those alike keep Unicode's order.
        found.sort_by(|(one, _), (other, _)| other.total_cmp(one));
        found
    }
}

/// How well the words typed find an emoji, or none where they don't. Each
/// word is asked as the start of a word and as a whole one.
fn score(emoji: &Emoji, asked: &[(String, String)], typed: &Typed) -> Option<f64> {
    let (mut total, mut named) = (0, 0);
    for (start, whole) in asked {
        if emoji.named.contains(whole.as_str()) {
            total += NAME;
            named += 1;
        } else if emoji.named.contains(start.as_str()) {
            total += NAME_START;
            named += 1;
        } else if emoji.meant.contains(whole.as_str()) {
            total += KEYWORD;
        } else if emoji.meant.contains(start.as_str()) {
            total += KEYWORD_START;
        } else {
            return emoji
                .name
                .score(typed)
                .map(|score| f64::from(score) / LOOSE_SCALE);
        }
    }
    let worth = WORTH_SCALE * f64::from(total);
    if named == 0 {
        return Some(worth);
    }
    Some(worth - LEFT_OVER * f64::from(emoji.words.saturating_sub(named)))
}

/// A text's words of letters and digits, lowered, a space apart, with a
/// space before and after, as Emoji.js spaced them: what isn't a letter
/// or digit of the alphabet parts two words.
fn spaced(text: &str) -> String {
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|one: char| !(one.is_ascii_lowercase() || one.is_ascii_digit()))
        .filter(|word| !word.is_empty())
        .collect();
    format!(" {} ", words.join(" "))
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emoji::records;
    use crate::rows::words;

    const LIST: &str = concat!(
        "[\"😀\", \"grinning face\", \"cheerful grin happy\", \"Smileys & Emotion\"]\n",
        "[\"😸\", \"grinning cat with smiling eyes\", \"cat grin smile\", \"Smileys & Emotion\"]\n",
        "[\"🐈\", \"cat\", \"pet\", \"Animals & Nature\"]\n",
        "[\"🎉\", \"party popper\", \"celebration tada\", \"Activities\"]\n",
    );

    fn index(list: &str) -> Index {
        Index::of(&records(list, &words()))
    }

    /// A list's index, with its records, so what's found is named as its
    /// row shows it rather than as the index folds it to match.
    struct Listed {
        index: Index,
        records: Vec<Value>,
    }

    fn listed(list: &str) -> Listed {
        let records = records(list, &words());
        Listed {
            index: Index::of(&records),
            records,
        }
    }

    fn asked(index: &Index, words: &str) -> Value {
        index.answer(&json!({ "ask": "search", "place": "emoji", "words": words }))
    }

    /// The names of what's found for `words`, best first, as each one's row
    /// says it; a key with no row reads as none, which no name matches.
    fn names(listed: &Listed, words: &str) -> Vec<String> {
        let answer = asked(&listed.index, words);
        let names = |key: &Value| {
            listed
                .records
                .iter()
                .map(|record| &record["row"])
                .find(|row| row["key"] == *key)
                .map(|row| text(&row["name"]).to_owned())
                .unwrap_or_default()
        };
        answer["keys"]
            .as_array()
            .into_iter()
            .flatten()
            .map(names)
            .collect()
    }

    #[test]
    fn a_name_whole_comes_before_one_with_words_left_over() {
        assert_eq!(
            names(&listed(LIST), "cat"),
            ["Cat", "Grinning cat with smiling eyes"]
        );
    }

    #[test]
    fn alike_they_keep_unicodes_order() {
        // Both names of two words, one of them started by what's typed, so
        // they score alike, and the list's order, not A to Z, puts the
        // face first.
        let tied = concat!(
            "[\"😀\", \"grinning face\", \"grin\", \"Smileys & Emotion\"]\n",
            "[\"😺\", \"grinning cat\", \"grin\", \"Smileys & Emotion\"]\n",
        );
        assert_eq!(
            names(&listed(tied), "grin"),
            ["Grinning face", "Grinning cat"]
        );
    }

    #[test]
    fn one_found_only_loosely_is_graded_so() {
        // "trex" starts no word of "t rex", and is found only as the
        // name's letters close up across its hyphen.
        let list = "[\"🦖\", \"T-Rex\", \"dinosaur\", \"Animals & Nature\"]\n";
        let listed = listed(list);
        let answer = asked(&listed.index, "trex");
        assert_eq!(names(&listed, "trex"), ["T-Rex"], "{answer}");
        assert_eq!(answer["grade"], 1, "found only loosely: {answer}");
    }

    #[test]
    fn words_past_the_longest_are_cut_there() {
        let index = index(LIST);
        let long = format!("party {}", "x".repeat(LONGEST));
        let answer = asked(&index, &long);
        assert_eq!(
            answer["words"].as_str().map(str::len),
            Some(long.len()),
            "its words come back as asked, so the shell takes the answer"
        );
        assert_eq!(answer["keys"], json!([]), "{answer}");
    }

    #[test]
    fn a_keyword_finds_what_an_emoji_means_marking_none_of_its_name() {
        let listed = listed(LIST);
        let answer = asked(&listed.index, "tada");
        assert_eq!(names(&listed, "tada"), ["Party popper"]);
        assert_eq!(answer["grade"], 0, "found by where a word starts: {answer}");
        assert_eq!(
            answer["marks"],
            json!({}),
            "its name holds none of tada: {answer}"
        );
    }

    #[test]
    fn a_name_found_has_its_letters_marked() {
        let answer = asked(&index(LIST), "party");
        assert_eq!(answer["marks"]["row:emoji:3"], json!([[0, 5]]), "{answer}");
    }

    #[test]
    fn nothing_typed_finds_nothing() {
        assert_eq!(asked(&index(LIST), " ")["keys"], json!([]));
    }

    #[test]
    fn within_a_group_it_finds_only_those_inside() {
        let index = index(LIST);
        let answer = index.answer(&json!({
            "ask": "search", "place": "emoji", "words": "cat", "within": "row:emoji:group:1",
        }));
        assert_eq!(answer["keys"], json!(["row:emoji:2"]), "{answer}");
    }

    #[test]
    fn it_answers_at_most_as_many_as_asked() {
        let answer = index(LIST).answer(&json!({
            "ask": "search", "place": "emoji", "words": "grin", "most": 1,
        }));
        assert_eq!(answer["keys"], json!(["row:emoji:0"]), "{answer}");
    }

    #[test]
    fn it_says_how_many_it_found_past_those_it_answers() {
        let answer = index(LIST).answer(&json!({
            "ask": "search", "place": "emoji", "words": "grin", "most": 1,
        }));
        assert_eq!(answer["found"], 2, "{answer}");
        let none = asked(&index(LIST), "zzzz");
        assert_eq!(none["found"], 0, "{none}");
    }

    #[test]
    fn another_place_is_refused_so_the_shell_matches_its_own() {
        let answer =
            index(LIST).answer(&json!({ "ask": "search", "place": "clipboard", "words": "a" }));
        assert_eq!(answer["error"]["code"], "unknown-place", "{answer}");
    }

    /// What each query puts first over the list the launcher is built with,
    /// which the-launcher-finds-emoji names in `EMOJI_LIST`; a build of the
    /// history alone has none, and leaves these to that check.
    #[test]
    fn the_built_list_puts_first_what_each_query_means() {
        let Some(list) = std::env::var_os("EMOJI_LIST") else {
            assert!(
                std::env::var_os("EMOJI_LIST_WANTED").is_none(),
                "the-launcher-finds-emoji gives EMOJI_LIST, and it never came"
            );
            return;
        };
        let list = std::fs::read_to_string(&list).expect("EMOJI_LIST names a readable list");
        let listed = listed(&list);
        assert!(
            listed.index.emoji.len() >= 1800,
            "only {} emoji were listed",
            listed.index.emoji.len()
        );
        let firsts = [
            ("fire", "Fire"),
            ("thumbs", "Thumbs up"),
            ("shrug", "Person shrugging"),
            ("clap", "Clapping hands"),
            ("red heart", "Red heart"),
            ("canada", "Flag: Canada"),
            ("tada", "Party popper"),
            ("party", "Party popper"),
            ("cat", "Cat"),
            ("rocket", "Rocket"),
            ("eyes", "Eyes"),
            ("skull", "Skull"),
            ("think", "Thinking face"),
            ("grin", "Grinning face"),
            ("joy", "Face with tears of joy"),
            ("100", "Hundred points"),
            ("warn", "Warning"),
            ("check", "Check mark"),
            ("smiling cat", "Grinning cat with smiling eyes"),
            ("kitty", "Cat face"),
        ];
        let wrong: Vec<String> = firsts
            .iter()
            .filter_map(|(query, first)| {
                let found = names(&listed, query);
                (found.first().map(String::as_str) != Some(*first)).then(|| {
                    format!(
                        "{query:?} put first {:?}, not {first:?}",
                        &found[..found.len().min(4)]
                    )
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
        for query in ["tada", "party pop"] {
            assert_eq!(
                asked(&listed.index, query)["grade"],
                0,
                "{query:?} is found by where words start"
            );
        }
        for query in ["zzzz", "qxj", "kitty litter box", "thup"] {
            let found = names(&listed, query);
            assert!(
                found.is_empty(),
                "{query:?} found {found:?}, but nothing should"
            );
        }
    }
}
