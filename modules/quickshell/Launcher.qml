// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return runs it, or Shift+Return opens it below the
// focused window. Tab takes the completion the command line offers. Each
// action shows its chord, so keys are learned by use.
//
// It opens as fast as niri can hand it the keys. Super+Space says "toggle"
// down a socket the shell keeps open (launcher-say, in home.nix), not
// through a second Quickshell that has to start first. Its surface stays
// mapped all along, a single clear pixel that takes neither the keys nor
// the pointer while it's closed. Opening only grows it, gives it the keys
// and shows the card, whose field already holds the focus. A letter typed
// straight after the chord then reaches the field, not the window below.
//
// What it finds comes from its providers, one file each in launcher/, in
// the order jitsusama.quickshell.launcher.providers lists them (rendered
// into Providers.qml), so a kind of thing is added or removed in one line.
//
// The right arrow at the end of the line steps into the selected row: what
// can be done to it, its verbs, searched the same way, and for a verb that
// needs one, its target. What's held shows as chips in the field; the left
// arrow, or Backspace with nothing typed, steps back out, and Escape always
// closes the launcher.
//
// What it finds is one row a thing, in an order that changes only when Joel
// changes it (launcher/Rank.js): the same letters land on the same row
// however the desktop changes around them, and the list keeps its height
// while it's open, so nothing moves under his eyes.
//
// It's the palette kind (docs/design.md, Shape and Kinds): the accent frame,
// since it holds the keys; a command line over a list, split by a hairline;
// every row a glyph in its column, a name, and its chord or what it is.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects
// Quickshell makes a folder's files types only once something imports it,
// and the providers are made from launcher/ by name, so it's imported here.
import qs.launcher
import "Fuzzy.js" as Fuzzy
import "launcher/Rank.js" as Rank

Scope {
    id: root

    required property var lock
    required property var grid
    property bool open: false

    // Wide enough for a long action and its chord at the large size.
    readonly property int width: 680
    // How many results show at once; the rest are a scroll away.
    readonly property int rows: 8

    readonly property string sparkle: String.fromCodePoint(0xf0674)
    readonly property string ghost: String.fromCodePoint(0xf02a0)

    // The providers, made from their files as the shell starts, each given
    // its settings and weight; one that fails to load is said and skipped.
    property var sources: []

    Component.onCompleted: {
        const made = [];
        for (const listed of Providers.providers) {
            const file = Qt.createComponent(`launcher/${listed.file}`);
            if (file.status !== Component.Ready) {
                console.error(`The launcher's ${listed.name} couldn't load: ${file.errorString()}`);
                continue;
            }
            const source = file.createObject(root, {
                settings: listed,
                weight: listed.weight,
                launcher: root
            });
            source.answered.connect(root.reanswer);
            source.changed.connect(() => root.refreshed(source));
            made.push(source);
        }
        sources = made;
    }

    // What the first provider to answer makes of what's typed, or null.
    // The selection starts on its headline, at -1, above the rows.
    property var answer: null
    // What the search found, below an answer's rows.
    property var found: []

    function answering(letters, fresh) {
        if (held.length > 0 || letters.trim() === "")
            return null;
        for (const source of sources) {
            const said = source.answer(letters);
            if (said)
                return said;
        }
        // Typed so far, a question's start asks it, as `question` says.
        const text = Fuzzy.plainly(letters);
        if (text.length < 3)
            return null;
        for (const source of sources) {
            const question = source.questions.find(asked => asked.startsWith(text));
            if (question === undefined)
                continue;
            const said = started(source, question, fresh);
            if (said)
                return Object.assign({}, said, { question: question });
        }
        return null;
    }

    // The question the answer was asked by when what's typed only starts
    // it, as "batt" starts "battery", or nothing. Its headline is chosen
    // only when nothing found starts a word the same way (compose), and
    // Tab finishes the question.
    readonly property string question: answer ? answer.question ?? "" : ""

    // Each letter of a question's start asks the same question, so its
    // answer is kept a moment rather than read again, as sensors are, on
    // every key. A late answer is always asked afresh.
    property var lastStarted: null
    function started(source, question, fresh) {
        const now = Date.now();
        const last = lastStarted;
        if (fresh && last && last.source === source && last.question === question && now - last.at < 2000)
            return last.said;
        const said = source.answer(question);
        lastStarted = { source: source, question: question, said: said, at: now };
        return said;
    }

    // Whether a name has a word starting with what's typed, as a row the
    // search found for that reason would.
    function startsAWord(name, letters) {
        const text = Fuzzy.plainly(letters);
        const folded = String(name).toLowerCase();
        for (let at = folded.indexOf(text); at >= 0; at = folded.indexOf(text, at + 1))
            if (at === 0 || !/[a-z0-9]/.test(folded[at - 1]))
                return true;
        return false;
    }

    // An answer's rows above what was found. Asked afresh, the selection
    // goes to the top; when a late answer arrives, a selection already
    // moved stays on the row it was on.
    function compose(fresh) {
        const before = answer ? answer.rows.length : 0;
        answer = answering(typed, fresh);
        const rows = answer ? answer.rows : [];
        // A thing shows once: a collection found whole stands for what's in
        // it, so neither its own rows, nor the things that live in it
        // (`inside` names it), nor a shortcut that opens it, whose chord its
        // row already draws, are listed beside it.
        const whole = new Set(found.filter(row => row.key.startsWith("collection:"))
            .map(row => row.opens));
        const once = row => row.key.startsWith("collection:")
            || !(whole.has(row.inside ?? "") || whole.has(row.opens ?? ""));
        // The row it was on, by what it is, so rows that come late, as a
        // collection's or an answer's do, never move it.
        const was = !fresh && current >= 0 && matches[current]
            ? Rank.identity(matches[current]) : "";
        // And a thing found more than one way is one row, as the one
        // with the most to say.
        const below = Rank.once(Rank.placed(found.filter(once),
            gathered.filter(row => !whole.has(row.from.settings.name)), picks.latched(typed)));
        matches = rows.concat(below);
        // An answer only started gives way to a name that starts the same.
        const meant = answer && !(answer.question && below.length > 0 && startsAWord(below[0].name, typed));
        if (fresh || (current === -1 && !answer))
            current = meant ? -1 : answer ? rows.length : 0;
        else if (current >= 0)
            current = Rank.pinned(matches, was, current + rows.length - before);
    }

    function reanswer() {
        if (open && held.length === 0)
            compose(false);
    }

    // The headline as a row of its own, for Return and the footer. Return
    // copies it, unless the answer says what it does instead, as setting
    // the brightness to what was typed does.
    readonly property var headline: !answer ? null : {
        name: answer.headline,
        // An answer still being asked for has nothing to copy yet.
        verb: answer.verb ?? ((answer.copy ?? answer.headline) === "" ? "close" : "copy"),
        now: true,
        note: answer.note ?? "",
        run: answer.run ?? (() => {
            const copy = answer.copy ?? answer.headline;
            if (copy !== "")
                Quickshell.clipboardText = copy;
        })
    }
    readonly property var selected: current === -1 ? headline : matches[current] ?? null

    // What sort of thing the selection is, in a word, while something is
    // typed: an answer, a collection, or its provider's kind. Inside a
    // collection or a thing's verbs, what's held already says it.
    readonly property string kind: {
        if (typed === "" || held.length > 0 || !selected)
            return "";
        if (selected === headline || (answer && answer.rows.includes(selected)))
            return "answer";
        // A collection's own row has no provider's kind; a shortcut that
        // opens one is still a shortcut.
        if (selected.what)
            return selected.what;
        return selected.opens ? "collection" : "";
    }

    // A collection's members stay out of the main search, but a thing it
    // offers beside them, such as turning Wi-Fi off, is found like any, and
    // so is the collection itself, by its title or any of its words, so
    // typing what it holds goes straight to it.
    // A desktop action another row stands for (Provider.qml's draws) is
    // found only as that row.
    readonly property var standIns: sources.reduce((all, source) => all.concat(source.draws), [])
    readonly property var choices: sources.reduce((all, source) => all.concat(source.things), [])
        .filter(row => !(row.key.startsWith("action:") && root.standIns.includes(row.name)))
        .map(withSynonyms)
        .concat(collections.map(named))
    readonly property var collections: sources.filter(source => source.collection)

    // A collection as a row the search finds: as it is at home, but for its
    // count, which would read all of it at every change.
    function named(source) {
        const collection = source.collection;
        const row = Object.assign(whole(source, false, false), { weight: source.weight });
        row.prepared = Fuzzy.prepare(row.name);
        row.keywords = Fuzzy.prepare([row.name, collection.noun].concat(collection.words ?? []).join(" "));
        // Its words are what a person calls it, so they count as its name.
        row.named = true;
        return row;
    }

    // A row for each collection that has matches: its best, with what's been
    // picked for this query before put first, and a count of the rest. Its
    // verbs are the matches, which the right arrow steps into with what was
    // typed kept. One letter finds too much of a collection to be worth a
    // row.
    property var gathered: []

    function gathering(letters) {
        if (letters.replace(/\s+/g, "").length < 2)
            return [];
        const rows = [];
        for (const source of collections) {
            // One opened only by its own chord adds no row to a search.
            if (source.collection.searched === false)
                continue;
            const said = source.gather(letters);
            const found = learnedFirst(said.rows, letters);
            if (found.length === 0)
                continue;
            const best = found[0];
            const noun = source.collection.noun;
            rows.push(Object.assign({}, best, {
                side: found.length > 1 ? `${found.length - 1} more ${noun}` : noun,
                chip: noun,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source,
                // Found by a word of its name, after the main search's own
                // (Rank.js's placed).
                tier: 1,
                verbs: () => found
            }));
        }
        return rows;
    }

    // How many of the likeliest picks the home screen shows, and what leads
    // them: jitsusama.quickshell.launcher.home.
    readonly property int likely: Providers.launcher.home.likely
    readonly property var pinned: Providers.launcher.home.pinned

    // How tall the list is while the launcher's open: as tall as home was
    // when it opened.
    property int tall: rows * Theme.space.xxxl

    // What kind of thing a row is, as it says at its right edge, by what
    // its provider calls its things: a closed set of nouns, so a kind is
    // learned once. An act or a set says what it is by its words, and an
    // answer's rows are the answer's, so they have none.
    // They're jitsusama.quickshell.launcher.kinds, so a machine can say them
    // its own way.
    readonly property var kinds: Providers.launcher.kinds

    function kindOf(row) {
        if (!row || row.opens || row.plain || (answer && answer.rows.includes(row)))
            return "";
        return kinds[row.what ?? ""] ?? "";
    }

    // The one word the foot gives Return: the row's own, or else an act's
    // first word ("forget", "disconnect"), which its name starts with, or
    // "open" for a thing.
    function verbOf(row) {
        if (!row)
            return "";
        if ((row.verb ?? "") !== "")
            return row.verb;
        return kindOf(row) !== "" ? "open" : Rank.firstWord(row.name ?? "") || "open";
    }

    // A chord as words, for the foot: its keys joined by +, those pressed
    // one after another a space apart, as Chord draws them.
    function chordText(keys) {
        return (keys ?? []).map((key, index) => index === 0 ? key.text
            : `${key.after === true ? " " : "+"}${key.text}`).join("");
    }

    // The word that found a row by what else it's called ("sleep" for
    // Suspend the machine), shown after its name, so a row is never named
    // by a word it doesn't say and Joel can see why it's there: the run of
    // its other words that each word typed starts, or the first one.
    function saidAs(row) {
        const typedWords = Fuzzy.words(typed);
        // Its keywords hold its name too, and a hyphened word of it closed
        // up, so a search can say some words of each; only the others are
        // shown.
        const named = new Set(Fuzzy.words(row ? row.name : "")
            .reduce((all, word) => all.concat([word, word.replace(/-/g, "")]), []));
        const own = row && row.keywords
            ? Fuzzy.words(row.keywords.text ?? "").filter(word => !named.has(word)) : [];
        const inName = word => Array.from(named).some(one => one.startsWith(word));
        const typedOwn = typedWords.filter(word => !inName(word));
        if (typedOwn.length === 0 || own.length === 0)
            return "";
        for (let at = 0; at + typedOwn.length <= own.length; at++) {
            if (typedOwn.every((word, i) => own[at + i].startsWith(word)))
                return own.slice(at, at + typedOwn.length).join(" ");
        }
        return own.find(word => word.startsWith(typedOwn[0])) ?? "";
    }

    // The fewest letters that reach a row from what's typed: the shortest
    // start of it that's learned to mean this row. Said in the foot when
    // it's shorter than what was typed, so the habit gets quicker by itself.
    function enough(row) {
        const letters = Fuzzy.plainly(typed);
        if (!row || letters.includes(" "))
            return "";
        for (let length = 1; length < letters.length; length++) {
            const start = letters.slice(0, length);
            if (picks.latched(start) === row.key)
                return start;
        }
        return "";
    }

    // The collections home keeps on its shelf, in this order, each with
    // the chord that opens it: the ones reached most. The rest are a word
    // away, and most have chords of their own.
    readonly property var shelf: Providers.launcher.home.shelf

    // A row's own words, with those a machine adds for it
    // (jitsusama.quickshell.launcher.synonyms), so a habit of Joel's finds
    // it. Made once for each row, and never twice for the same one.
    function withSynonyms(row) {
        const more = Providers.launcher.synonyms[row.key];
        if (!more || row.synonymised)
            return row;
        row.keywords = Fuzzy.prepare([row.keywords ? row.keywords.text : ""].concat(more)
            .join(" ").trim());
        row.synonymised = true;
        return row;
    }

    // How often each thing's been picked, taken as the launcher opens, so
    // what it orders holds still while it's open.
    property var often: ({})

    // With nothing typed, a shelf that's always the same shape: what's in
    // front of you, then what holds while it's on (a mode, a screenshot
    // just taken), then the likeliest picks, most picked first, and after a
    // gap the shelf's collections, each its size and the chord that opens
    // it, so the chords are learned by seeing them where they're used.
    function home() {
        const rows = [];
        const shown = new Set();
        for (const source of sources)
            for (const row of source.home()) {
                rows.push(row);
                shown.add(Rank.identity(row));
            }
        const byKey = {};
        const names = {};
        for (const choice of choices) {
            byKey[choice.key] = choice;
            names[choice.key] = choice.name;
        }
        let added = 0;
        // Those pinned lead, in their order, whether picked or not.
        for (const key of pinned.concat(Rank.likeliest(often, names))) {
            const choice = byKey[key];
            // A collection is always below with the rest, in its own place,
            // so it isn't listed again as a likely pick.
            if (!choice || shown.has(Rank.identity(choice)) || choice.named)
                continue;
            rows.push(choice);
            shown.add(Rank.identity(choice));
            if (++added === likely)
                break;
        }
        const kept = shelf.map(name => collections.find(source => source.settings.name === name))
            .filter(source => !!source);
        return rows.concat(kept.map((source, index) => whole(source, index === 0
            && rows.length > 0)));
    }

    // A collection as one row at home: Return or the right arrow opens all
    // of it.
    function whole(source, gap, counted = true) {
        const collection = source.collection;
        const name = source.settings.name;
        const opener = Shortcuts.actions.find(action => action.opens === name);
        return {
            key: `collection:${name}`,
            verb: "open",
            name: collection.title ?? collection.noun,
            plain: true,
            count: counted ? source.counted() : "",
            glyph: collection.glyph,
            hue: "voice",
            chord: opener ? opener.chord : [],
            side: "",
            gap: gap,
            opens: name,
            from: source,
            chip: collection.noun,
            ask: collection.ask,
            lead: collection.glyph,
            leadHue: "voice",
            verbs: () => source.everything()
        };
    }

    // Something to try, a different one each time the launcher opens, taken
    // in turn from each provider that has some, so every kind gets a turn.
    readonly property var examples: {
        const out = [];
        const longest = Math.max(0, ...sources.map(source => source.examples.length));
        for (let i = 0; i < longest; i++)
            for (const source of sources)
                if (i < source.examples.length)
                    out.push(source.examples[i]);
        return out;
    }
    property int tried: -1
    readonly property string example: examples.length === 0 ? ""
        : examples[Math.max(0, tried) % examples.length]
    readonly property bool atHome: typed === "" && held.length === 0

    // A collection's matches with those picked before for what's typed
    // first, most picked first, the rest in their order.
    function learnedFirst(found, letters) {
        const learned = picks.weights(letters);
        const before = found.filter(row => learned[row.key]);
        if (before.length === 0)
            return found;
        before.sort((a, b) => learned[b.key].weight - learned[a.key].weight);
        return before.concat(found.filter(row => !learned[row.key]));
    }

    // Shift+Return, as a chord beside a verb that does what it does.
    readonly property var belowChord: [
        { text: "\u21e7", wide: false },
        { text: "\u21b5", wide: false }
    ]

    // What's been stepped into, each with the rows it offers and where the
    // step before it was left: what was typed, the row selected and how far
    // the list was scrolled, so stepping back out finds it as it was. A
    // thing and its verbs, then a verb and its targets.
    property var held: []

    // Where the launcher is now, for a step into something to keep.
    function here() {
        const row = selected;
        return {
            typed: typed,
            at: current,
            key: row ? Rank.identity(row) : "",
            scroll: list.contentY
        };
    }

    // Selects what was selected before, by what it is when it's still
    // there, so of two windows titled alike it's the same one, since what
    // was searched again may have moved it, or else by where it was, and
    // scrolls the list back to where it stood.
    function reselect(left) {
        const byKey = left.key === "" ? -1
            : matches.findIndex(row => Rank.identity(row) === left.key);
        const lowest = answer ? -1 : 0;
        const at = left.at === -1 && answer ? -1
            : byKey >= 0 ? byKey : Math.max(lowest, Math.min(left.at, matches.length - 1));
        list.contentY = left.scroll;
        current = at;
        if (current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
    }
    readonly property var offered: held.length > 0 ? held[held.length - 1].rows : []

    // Whether the step held last is a collection, whose members are things
    // with verbs of their own, as at the top; or a row that says its verbs
    // are things too (nested), as a report's rows are, so stepping goes on
    // down as far as it holds anything.
    readonly property bool collecting: held.length > 0 && collects(held[held.length - 1])

    function collects(step) {
        return !!step.row.from || !!step.row.nested;
    }

    // Steps into a row: a thing's verbs, or a verb's targets. Says whether
    // there was anywhere to go.
    function enter(row) {
        if (!row)
            return false;
        if (held.length > 0 && row.given) {
            ask(row);
            return true;
        }
        const into = held.length === 0 || collecting ? (row.verbs ? row.verbs() : null)
            : row.targets ? row.targets() : null;
        if (!into || into.length === 0)
            return false;
        held = held.concat([Object.assign(here(), { row: row, rows: into })]);
        if (row.from)
            row.from.opened();
        // A collection is stepped into with what was typed still searching it.
        restart(row.from ? typed : "");
        return true;
    }

    // What the field is asking for words for, such as a password: the step
    // held last, when its row hands what's typed to `given`.
    readonly property var asking: held.length > 0 && held[held.length - 1].row.given
        ? held[held.length - 1].row : null

    // Asks for words in the field, with nothing to choose from below it.
    function ask(row) {
        held = held.concat([Object.assign(here(), { row: row, rows: [] })]);
        restart("");
    }

    // Hands over what was typed, and empties the field for another try
    // while the footer says how it went.
    function give() {
        const words = prompt.text;
        if (words === "")
            return;
        asking.given(words);
        prompt.text = "";
    }

    // A collection whose things changed while it's open is searched again,
    // the selection kept on the row it was on, and so is anything stepped
    // into below it, each step's rows asked for again, since they can say
    // how a thing stands, as a Wi-Fi network's say its access points.
    function refreshed(source) {
        if (!open || asking)
            return;
        const from = held.findIndex(step => step.row.from === source);
        if (from < 0)
            return;
        const key = selected ? selected.key : null;
        const steps = held.slice(0, from + 1);
        let rows = source.everything();
        for (const step of held.slice(from + 1)) {
            const thing = rows.find(row => row.key === step.row.key);
            // Asked as enter asks: a collection's things, or a nested row's,
            // for their verbs, anything else for its targets.
            const into = !thing ? null
                : collects(steps[steps.length - 1]) ? (thing.verbs ? thing.verbs() : null)
                : thing.targets ? thing.targets() : null;
            if (!into)
                return;
            steps.push(Object.assign({}, step, { row: thing, rows: into }));
            rows = into;
        }
        if (held.length > from + 1)
            held = steps;
        search(typed);
        const at = matches.findIndex(row => row.key === key);
        if (at > 0)
            current = at;
    }

    function back() {
        if (held.length === 0)
            return;
        const left = held[held.length - 1];
        held = held.slice(0, -1);
        restart(left.typed);
        reselect(left);
    }

    // Searches afresh for what the field is set to, since what was found
    // before belongs to another step.
    function restart(text) {
        typed = "";
        candidates = [];
        if (prompt.text === text)
            search(text);
        else
            prompt.text = text;
    }

    property string typed: ""
    property var matches: []
    property int current: 0
    property var pending: null

    // The words typed, and every choice they found before the finest were
    // kept: a longer query can only find fewer, so it searches these.
    property var words: []
    property var candidates: []

    // What was picked for what was typed, which the ranking leans on.
    Picks {
        id: picks
        learns: Providers.learning.enable
        halfLife: Providers.learning.halfLife
        remember: Providers.learning.remember
    }

    // Every choice the words find (Fuzzy.js says how), in Rank.js's order:
    // what's latched for what's typed, then by tier, then what was picked
    // for it, what's picked most and the better match. One found only with
    // its keywords, beside its name or alone, is in the last tier, but for a
    // collection's own words, which are what it's called.
    function search(letters) {
        const narrowing = typed !== "" && letters.startsWith(typed);
        typed = letters;
        words = Fuzzy.words(letters);
        if (asking) {
            matches = [];
            answer = null;
            current = 0;
            return;
        }
        if (held.length > 0) {
            const from = held[held.length - 1].row.from;
            // A collection searches itself, all of it until something's typed.
            if (from)
                matches = words.length === 0 ? from.everything()
                    : learnedFirst(from.search(letters).rows, letters);
            // A step's rows, all of them in their order until something's
            // typed, then those it finds, best first.
            else if (words.length === 0)
                matches = offered.slice();
            else {
                const found = [];
                for (const row of offered) {
                    const m = Fuzzy.match(row.prepared, words, false);
                    if (m.score >= 0)
                        found.push({
                            item: row.prepared,
                            score: m.score,
                            grade: m.grade,
                            choice: row
                        });
                }
                matches = Fuzzy.keep(found).map(f => f.choice);
            }
            answer = null;
            current = 0;
            list.positionViewAtBeginning();
            return;
        }
        gathered = gathering(letters);
        if (words.length === 0) {
            candidates = [];
            root.found = home();
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            for (const choice of narrowing ? candidates : choices) {
                const byName = (choice.prepared.mask & need) === need
                    ? Fuzzy.match(choice.prepared, words, false) : { score: -1 };
                if (byName.score >= 0) {
                    found.push({
                        item: choice.prepared,
                        score: byName.score * choice.weight,
                        grade: byName.grade,
                        tier: Rank.tier(choice.name, letters, false),
                        choice: choice
                    });
                    continue;
                }
                if (!choice.keywords || (choice.keywords.mask & need) !== need)
                    continue;
                const byWord = Fuzzy.match(choice.keywords, words, false);
                if (byWord.score >= 0)
                    found.push({
                        item: choice.prepared,
                        score: byWord.score * choice.weight / (choice.named ? 1 : 4),
                        grade: byWord.grade,
                        tier: choice.named ? 1 : 2,
                        choice: choice,
                        byWord: true
                    });
            }
            candidates = found.map(f => f.choice);
            const learned = picks.weights(letters);
            const ordered = Rank.order(found, {
                latched: picks.latched(letters),
                picked: learned,
                often: often
            });
            root.found = ordered.map(f => Object.assign({
                byWord: f.byWord === true,
                times: (learned[f.choice.key] ?? { times: 0 }).times,
                grade: f.grade,
                tier: f.tier,
                kept: !!learned[f.choice.key]
            }, f.choice));
        }
        compose(true);
        list.positionViewAtBeginning();
    }

    onCurrentChanged: {
        if (current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
    }

    // The rest of the selected name from where what's typed starts a word in
    // it, so the command line can finish the thought.
    readonly property string completion: {
        // An answer that does something says what, rather than equalling it.
        if (current === -1 && answer) {
            const rest = answer.question ? answer.question.slice(Fuzzy.plainly(typed).length) : "";
            return rest + (answer.run ? ` \u2192 ${answer.headline}` : ` = ${answer.headline}`);
        }
        const choice = matches[current];
        if (!choice || typed === "")
            return "";
        const name = choice.name;
        const at = name.toLowerCase().indexOf(typed.toLowerCase());
        if (at < 0 || (at > 0 && name[at - 1] !== " "))
            return "";
        return name.slice(at + typed.length);
    }

    // A name with the letters that matched brought forward, as styled text.
    // Worked out only for the rows drawn.
    function marked(choice) {
        const escape = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        const at = choice.byWord || choice.plain ? []
            : Fuzzy.match(choice.prepared, words, true).positions ?? [];
        let out = "";
        for (let i = 0; i < choice.name.length; i++) {
            const letter = escape(choice.name[i]);
            out += at.includes(i)
                ? `<font color="${Theme.strong}">${letter}</font>`
                : letter;
        }
        return out;
    }

    function run(choice, below) {
        if (!choice) {
            open = false;
            return;
        }
        // A verb that needs a target asks for it, and one that needs words
        // asks for them.
        if (held.length > 0 && choice.targets) {
            enter(choice);
            return;
        }
        if (choice.given) {
            ask(choice);
            return;
        }
        // One that can't be taken back, such as powering off, steps into
        // its verbs, which say what it does, so a second Return does it and
        // Escape or the left arrow backs out.
        if (choice.confirm && held.length === 0 && enter(choice))
            return;
        // One whose outcome takes a while keeps the launcher, which says how
        // it's going.
        if (choice.stays) {
            choice.run();
            return;
        }
        // One that opens a collection steps into it, the launcher kept open.
        if (choice.opens) {
            picks.picked(typed, choice.key);
            into(choice.opens);
            return;
        }
        // The clipboard takes text only from a program holding the keys.
        if (choice.now)
            choice.run();
        open = false;
        // Acting on a thing says it was what was meant by what was typed;
        // in a collection, the one picked from it was.
        const first = held.length > 0 ? held[0] : { row: choice, typed: typed };
        const meant = first.row.from ? choice : first.row;
        // One that can't be taken back is latched only once its first word's
        // been typed, so a slip of two letters never powers off.
        picks.picked(first.row.from ? typed : first.typed, meant.key,
            meant.confirm ? Rank.firstWord(meant.name).length : 1);
        if (choice.now)
            return;
        if (below && choice.below)
            choice.below();
        else if (choice.later) {
            pending = choice;
            later.restart();
        } else
            choice.run();
    }

    Timer {
        id: later
        interval: Theme.motion.short
        onTriggered: {
            root.pending.run();
            root.pending = null;
        }
    }

    // What a key says down the socket, a line each: "toggle", or
    // "collection" and a collection's name. Each connection is closed from
    // this end once its line is read, so launcher-say returns at once and
    // Quickshell has no peer's hang-up to warn of.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/launcher.sock`
        handler: Socket {
            id: caller

            parser: SplitParser {
                onRead: line => {
                    caller.connected = false;
                    const words = line.trim().split(/\s+/);
                    if (words[0] === "toggle")
                        root.open = !root.open;
                    else if (words[0] === "collection")
                        root.into(words.slice(1).join(" "));
                    else
                        console.warn(`The launcher was told "${line}", which it doesn't know.`);
                }
            }
        }
    }

    IpcHandler {
        target: "launcher"

        function toggle(): void {
            root.open = !root.open;
        }

        // Opens the launcher into a collection by its provider's name.
        function collection(name: string): void {
            root.into(name);
        }

        function showing(): string {
            return root.showing();
        }
    }

    // What the launcher shows, as JSON: whether it holds the keyboard,
    // what's typed and held, the first twenty rows' names in order (a
    // collection holds thousands), and the row Return
    // takes, with its chord and whether an answer is still being asked for. A script waits on this before it
    // presses a key, rather than guessing how long the launcher takes.
    function showing() {
        const row = root.selected;
        return JSON.stringify({
            open: root.open,
            keys: root.open && prompt.Window.active,
            typed: root.typed,
            held: root.held.map(step => step.row.chip ?? step.row.name),
            rows: root.matches.slice(0, 20).map(row => row.name),
            // What each of those rows is, and how it reads whole, so a
            // test can say no thing is listed twice.
            things: root.matches.slice(0, 20).map(row => ({
                is: Rank.identity(row),
                reads: [row.name, row.count ?? "", row.side ?? "", root.kindOf(row)]
                    .filter(part => part !== "").join(" \u00b7 ")
            })),
            asking: !!root.asking,
            selected: !row ? null : {
                name: row.name,
                is: Rank.identity(row),
                verb: root.verbOf(row),
                chord: (row.chord ?? []).map(key => key.text).join(" "),
                side: row.side ?? "",
                settled: row !== root.headline || !root.answer.asking
            },
            kind: root.kind,
            footer: foot.glancing ? "" : note.said,
            glance: foot.glancing ? standing.said : ""
        });
    }

    // Said each time it changes, while reports are on, so the test hears
    // every step rather than asking over and over.
    readonly property string shown: Options.reports ? showing() : ""

    onShownChanged: Report.say(`showing ${shown}`)

    // Opens straight into a collection, all of it shown until something's
    // typed; the left arrow steps out to the search. A name no collection
    // has opens the search. A collection may lead with one member, which
    // opens on its actions, as the notices do for one still on screen.
    function into(name) {
        open = true;
        const source = collections.find(s => s.settings.name === name);
        if (!source)
            return;
        held = [Object.assign(here(), {
            row: {
                key: "",
                name: source.collection.noun,
                chip: source.collection.noun,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source
            },
            rows: []
        })];
        source.opened();
        restart("");
        const lead = source.leading ? source.leading() : null;
        if (lead)
            enter(lead);
    }

    // Where the pointer last was, in the window, so the selection follows
    // it only when it moves: a launcher opening under a resting pointer, or
    // a list scrolling under one, leaves the selection where the keys put it.
    property point pointer: Qt.point(-1, -1)

    function pointed(index, at) {
        const moved = pointer.x >= 0 && (at.x !== pointer.x || at.y !== pointer.y);
        pointer = at;
        if (moved)
            current = index;
    }

    onOpenChanged: {
        if (open) {
            pointer = Qt.point(-1, -1);
            held = [];
            tried += 1;
            often = picks.counts();
            restart("");
            // Home's height, which the list keeps until it closes.
            const shown = Math.max(1, Math.min(matches.length, rows));
            tall = shown * Theme.space.xxxl + (matches.slice(0, shown).some(row => row.gap)
                ? Theme.space.m : 0);
            // Before niri hands over the keys, so the first one lands.
            prompt.take();
            Report.say(`stands ${standing.said}`);
        } else {
            screenHere = focusedScreen;
        }
    }

    // The screen the keys are on, followed while the launcher is closed so
    // its surface is already there when Super+Space comes, and kept while
    // it's open so it never jumps from under Joel's eyes.
    readonly property var focusedScreen: {
        Niri.revision;
        const workspace = Niri.workspace();
        return Quickshell.screens.find(screen => workspace && screen.name === workspace.output)
            ?? Quickshell.screens[0] ?? null;
    }
    property var screenHere: focusedScreen

    onFocusedScreenChanged: {
        if (!open)
            screenHere = focusedScreen;
    }

    // Nothing at all: the pointer's reach and niri's blur while closed.
    Region {
        id: nowhere
    }

    // Mapped all along, so opening makes no surface and draws no first
    // frame before niri hands over the keys. While closed it's one pixel at
    // the screen's top edge rather than the card's size, since niri draws a
    // surface kept out of screencasts (config.kdl) as a black box the size
    // of the whole surface, shown or not.
    PanelWindow {
        visible: true
        screen: root.screenHere
        anchors.top: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: root.open && screen // qmllint disable unqualified
            ? screen.height / 4 - card.reach : 0 // qmllint disable unqualified
        implicitWidth: root.open ? root.width + 2 * card.reach : 1
        implicitHeight: root.open ? card.height + 2 * card.reach : 1
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: root.open ? card.area : nowhere
        BackgroundEffect.blurRegion: root.open ? card.area : nowhere
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: root.open ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        WlrLayershell.namespace: "launcher"

        Floating {
            id: card
            x: reach
            y: reach
            width: root.width
            keys: true
            // Clear rather than hidden while closed, since a hidden field
            // can't hold the focus, and a key that came before the card was
            // shown would be lost.
            opacity: root.open ? 1 : 0

            Column {
                width: parent.width

                // What's typed, an inset inside the frame.
                Item {
                    width: parent.width
                    height: prompt.height + 2 * Theme.inset

                    Prompt {
                        id: prompt
                        x: Theme.inset
                        y: Theme.inset
                        width: parent.width - 2 * Theme.inset
                        // A thing held leads the line with its own glyph.
                        readonly property var thing: root.held.length > 0
                            ? root.held[0].row : null
                        // A secret is typed beside the padlock.
                        secret: root.asking !== null && root.asking.secret === true
                        glyph: secret ? String.fromCodePoint(0xf023)
                            : thing ? thing.lead ?? thing.glyph : String.fromCodePoint(0xf0349)
                        hue: secret ? Theme.machine
                            : thing ? Theme[thing.leadHue ?? thing.hue] : Theme.muted
                        glyphWidth: Theme.space.xxl
                        chips: root.held.map(step => ({ text: step.row.chip ?? step.row.name }))
                        hint: root.held.length === 0 ? "what next?"
                            : root.held[root.held.length - 1].row.ask ?? "what to do?"
                        ghost: root.completion
                        // At home, something to try; elsewhere, where the
                        // selection is, once there's more than fits.
                        // The row says what it is, so this doesn't.
                        aside: root.atHome && root.example !== "" ? `try  ${root.example}`
                            : root.matches.length > root.rows && root.current >= 0
                                ? `${root.current + 1} of ${root.matches.length}` : ""
                        onTextChanged: root.search(text)
                        onAccepted: below => root.asking ? root.give()
                            : root.run(root.selected, below)
                        // Tab on an answer puts the answer in place of the
                        // question, so a sum can be carried on; one still
                        // being asked for leaves the question as it is.
                        onCompleted: {
                            // At home, Tab takes the suggestion to try.
                            if (root.atHome && root.example !== "")
                                text = root.example;
                            else if (root.current !== -1 || !root.answer)
                                text = text + root.completion;
                            // A question only started is finished first.
                            else if (root.question !== "")
                                text = root.question;
                            // An answer that does something has nothing to
                            // put in the question's place.
                            else if (!root.answer.run && (root.answer.copy ?? root.answer.headline) !== "")
                                text = root.answer.copy ?? root.answer.headline;
                        }
                        onCancelled: root.open = false
                        onForward: root.enter(root.matches[root.current])
                        onBackward: root.back()
                        // The highest the selection goes: an answer's headline
                        // sits above the rows, at -1.
                        readonly property int highest: root.answer ? -1 : 0
                        Keys.onUpPressed: root.current = Math.max(root.current - 1, highest)
                        Keys.onDownPressed: root.current
                            = Math.max(highest, Math.min(root.current + 1, root.matches.length - 1))
                        Keys.onPressed: event => {
                            if (event.key === Qt.Key_PageUp)
                                root.current = Math.max(root.current - root.rows, highest);
                            else if (event.key === Qt.Key_PageDown)
                                root.current
                                    = Math.min(root.current + root.rows, root.matches.length - 1);
                        }
                    }
                }

                // What it reaches, split from what's typed by a hairline.
                Rectangle {
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                Column {
                    x: Theme.inset
                    width: parent.width - 2 * Theme.inset
                    topPadding: Theme.inset
                    bottomPadding: Theme.inset

                    // As tall as it opened, whatever's found, so the foot
                    // never jumps and a row never moves because others
                    // came or went: the rows it doesn't fill are space.
                    Item {
                        width: parent.width
                        height: root.tall

                        ListView {
                            id: list

                            anchors.fill: parent
                            // A gap sets the collections apart at home.
                            readonly property int apart: root.matches.some(row => row.gap)
                                ? Theme.space.m : 0
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            model: root.matches

                            delegate: Item {
                                id: entry

                                required property var modelData
                                required property int index

                                width: list.width
                                height: Theme.space.xxxl + (modelData.gap ? list.apart : 0)

                                Choice {
                                    anchors.bottom: parent.bottom
                                    selected: entry.index === root.current
                                    glyph: entry.modelData.glyph
                                    // A colour's swatch wears the colour itself.
                                    hue: entry.modelData.swatch ?? Theme[entry.modelData.hue]
                                    name: selected ? entry.modelData.name
                                        : root.marked(entry.modelData)
                                    chord: entry.modelData.chord
                                    side: entry.modelData.side
                                    count: entry.modelData.count ?? ""
                                    kind: root.held.length === 0 ? root.kindOf(entry.modelData) : ""
                                    also: entry.modelData.byWord ? root.saidAs(entry.modelData) : ""
                                    onClicked: root.run(entry.modelData, false)
                                    onPointed: at => root.pointed(entry.index, at)
                                }
                            }
                        }

                        // With nothing typed, the shell says what it can do, or an
                        // empty collection says it's empty in its own words; with
                        // nothing found, it says so. A field asking for words has
                        // only space below it.
                        Choice {
                            readonly property var inside: root.held.length > 0
                                ? root.held[root.held.length - 1].row.from ?? null : null
                            visible: root.matches.length === 0 && root.answer === null
                                && root.asking === null
                            glyph: root.typed === "" ? root.sparkle : root.ghost
                            hue: Theme.voice
                            quiet: true
                            name: root.typed !== ""
                                ? `Nothing answers to \u201c${root.typed}\u201d.`
                                : inside?.collection
                                    ? inside.collection.empty ?? `No ${inside.collection.noun} yet.`
                                : "A window, a program, an action, or what a key does."
                        }
                    }

                    // A picture copied, as it is, while its row is selected:
                    // in its own shape, no wider than the rows nor taller
                    // than six of them, aligned with the footer's words, its
                    // corners a part's, and the raised colour until it's
                    // drawn, as a song's cover is in a notice.
                    Item {
                        id: preview

                        readonly property var row: root.selected
                        readonly property string picture: row ? row.picture ?? "" : ""
                        readonly property real room: width - 2 * Theme.space.l
                        readonly property real most: 6 * Theme.space.xxxl
                        // Its own width and height, or a square for one whose
                        // type didn't say them.
                        readonly property real across: row && row.across > 0 ? row.across : most
                        readonly property real down: row && row.down > 0 ? row.down : most
                        readonly property real shrunk: Math.min(1, room / across, most / down)

                        width: parent.width
                        height: visible ? frame.height + Theme.space.m : 0
                        visible: picture !== ""

                        Rectangle {
                            id: frame
                            x: Theme.space.l
                            y: Theme.space.m
                            width: Math.round(preview.across * preview.shrunk)
                            height: Math.round(preview.down * preview.shrunk)
                            radius: Theme.inner
                            color: drawn.status === Image.Ready ? "transparent" : Theme.raised

                            Image {
                                id: drawn
                                anchors.fill: parent
                                source: preview.picture === "" ? "" : `file://${preview.picture}`
                                asynchronous: true
                                fillMode: Image.PreserveAspectFit
                                sourceSize.width: 2 * width
                                sourceSize.height: 2 * height
                                layer.enabled: true
                                layer.effect: MultiEffect {
                                    maskEnabled: true
                                    maskSource: corners
                                }
                                onStatusChanged: {
                                    if (status === Image.Ready)
                                        Report.say(`showed ${preview.row.name}`);
                                }
                            }

                            Rectangle {
                                id: corners
                                anchors.fill: parent
                                radius: Theme.inner
                                visible: false
                                layer.enabled: true
                            }
                        }
                    }

                    // The foot, one line, the one place that teaches, so the
                    // rows stay clean. On the right, what Return does here,
                    // Shift+Return where it can, and where the arrows step.
                    // On the left, what the selection has to say: why it's
                    // first when learning put it there, where a held thing
                    // is, where an answer came from; and when it says
                    // nothing, how things stand, which is the launcher's
                    // glance at the desktop.
                    Item {
                        id: foot

                        readonly property var choice: root.selected ?? root.asking
                        // The glance while nothing's typed, held or asked,
                        // and whenever the selection has nothing to say.
                        readonly property bool glancing: note.said === ""
                            || (root.typed === "" && root.held.length === 0 && root.asking === null)
                        // The room the left has, short of the keys.
                        readonly property real room: keys.x - Theme.space.xl - Theme.space.l

                        width: parent.width
                        height: Theme.space.xxl + Theme.space.s

                        Text {
                            id: note
                            x: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            width: keys.x - x - Theme.space.xl
                            elide: Text.ElideRight

                            // An answer's footer says where it came from.
                            readonly property string said: !foot.choice ? ""
                                : foot.choice.state ? foot.choice.state()
                                : (foot.choice.note ?? "") !== "" ? foot.choice.note
                                : foot.choice.plain && root.answer ? root.answer.note ?? ""
                                : root.held.length > 0 ? root.held[0].row.context ?? ""
                                // A thing's row says its kind where a chord
                                // would be, so its chord is taught here.
                                // What's typed is longer than it need be.
                                : root.enough(foot.choice) !== ""
                                    ? `${root.enough(foot.choice)} is enough`
                                : root.kindOf(foot.choice) !== ""
                                    && (foot.choice.chord ?? []).length > 0
                                    ? `next time ${root.chordText(foot.choice.chord)}`
                                : ""
                            // What it said last, kept while it fades out.
                            property string drawn: ""
                            onSaidChanged: {
                                if (note.said !== "")
                                    note.drawn = note.said;
                            }
                            Component.onCompleted: {
                                note.drawn = note.said;
                            }

                            text: note.drawn
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                            opacity: foot.glancing ? 0 : 1

                            Behavior on opacity {
                                NumberAnimation {
                                    duration: Theme.motion.short
                                }
                            }
                        }

                        // How things stand, the quietest thing here: the
                        // facts the margin's pieces hold that are always
                        // true (the time, the date, the battery, the
                        // network, the volume, the day's AI spend, pi),
                        // each its glyph and the least that tells Joel
                        // something, as a key hint is its key and what it
                        // does, at the hints' size and step, all muted
                        // but a fact that wants him, which wears its
                        // colour. At home it's the whole glance; with
                        // something typed, held or asked, only what never
                        // gives way and what wants him, so it never pulls
                        // the eye from the rows. The rest give way by
                        // collapse rank, lowest first, as room runs short.
                        Row {
                            id: standing

                            readonly property bool home: root.typed === "" && root.held.length === 0
                                && root.asking === null
                            readonly property var facts: Facts.all
                                .filter(fact => fact.entry.tier !== "when" && fact.piece
                                    && fact.piece.present && fact.piece.value !== ""
                                    && (home || fact.piece.wants || fact.entry.collapse === null))
                                .map(fact => {
                                    const sign = fact.piece.sign !== "" ? Theme.glyph[fact.piece.sign] ?? "" : "";
                                    const told = words => [sign, words].filter(part => part !== "").join(" ");
                                    return {
                                        entry: fact.entry,
                                        said: told(fact.piece.glance),
                                        least: told(fact.piece.least),
                                        piece: fact.piece
                                    };
                                })
                                .filter(fact => fact.said !== "")
                            // Short of room, every fact says its least before
                            // any gives way.
                            readonly property var kept: {
                                const wide = facts => facts.reduce((sum, fact) => sum + measure.advanceWidth(fact.said), 0)
                                    + Math.max(0, facts.length - 1) * spacing;
                                if (wide(facts) <= foot.room)
                                    return facts;
                                let kept = facts.map(fact => Object.assign({}, fact, { said: fact.least }));
                                for (;;) {
                                    const ranks = kept.map(fact => fact.entry.collapse)
                                        .filter(rank => rank !== null && rank !== undefined);
                                    if (wide(kept) <= foot.room || ranks.length === 0)
                                        return kept;
                                    const lowest = Math.min(...ranks);
                                    kept = kept.filter(fact => fact.entry.collapse !== lowest);
                                }
                            }
                            // As it's drawn, for a test.
                            readonly property string said: kept.map(fact => fact.said).join("  ")

                            x: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: keys.spacing
                            opacity: foot.glancing ? 1 : 0

                            Behavior on opacity {
                                NumberAnimation {
                                    duration: Theme.motion.short
                                }
                            }

                            FontMetrics {
                                id: measure
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                                font.features: { "tnum": 1 }
                            }

                            Repeater {
                                model: standing.kept

                                Text {
                                    required property var modelData

                                    text: modelData.said
                                    textFormat: Text.PlainText
                                    // As the margin's mark wears it (Margin.qml).
                                    color: !modelData.piece.wants ? Theme.muted
                                        : modelData.piece.role !== "text" ? Theme[modelData.piece.role]
                                        : Theme[modelData.piece.hue]
                                    font: measure.font
                                }
                            }
                        }

                        KeyHints {
                            id: keys
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            keys: {
                                const choice = foot.choice;
                                if (!choice)
                                    return [];
                                const keys = [["\u21b5", root.verbOf(choice)]];
                                if (root.held.length === 0 && choice.below)
                                    keys.push(["\u21e7\u21b5", "below"]);
                                if ((root.held.length === 0 || root.collecting) && choice.verbs)
                                    keys.push(["\u2192", choice.from ? "see all" : "more"]);
                                if (root.held.length > 0)
                                    keys.push(["\u2190", "back"]);
                                if (root.atHome && root.example !== "")
                                    keys.push(["\u21e5", "try it"]);
                                return keys;
                            }
                        }
                    }
                }
            }
        }
    }
}
