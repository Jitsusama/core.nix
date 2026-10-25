// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return runs it, or Shift+Return opens it below the
// focused window. Tab takes the completion the command line offers, and at
// home goes to the categories listed last, to browse what there is. Each
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
// What everything is called is the vocabulary's (Words.qml): its names,
// the words that find it, its parts, and the categories it's in. A
// category is a level too, found by its word, and its word and a space
// step into it, what's typed then searching only inside it; commands is
// the one listing every typed command, Tab putting its start in the field.
//
// What it finds is one row a thing, in an order that changes only when Joel
// changes it (launcher/Rank.js): the same letters land on the same row
// however the desktop changes around them, a list that changes while it's
// shown keeps its order (launcher/Steady.js), and the list keeps its height
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
import "launcher/Standing.js" as Standing
import "launcher/Steady.js" as Steady

Scope {
    id: root

    required property var lock
    required property var grid
    property bool open: false

    // Wide enough for a long action and its chord at the large size.
    readonly property int width: 680
    // How many results show at once; the rest are a scroll away.
    readonly property int rows: 8

    // The vocabulary (Words.qml), handed to the providers' common code
    // (Provider.qml), which can't import qs itself: Provider is the type
    // every provider is made from, and importing the shell's module there
    // made Machine's "what's running" answer nothing in the desktop test,
    // with no warning, as if its Jobs were another, unfed one.
    readonly property var vocabulary: Words
    // The theme's marks, handed over the same way, for a glyph Provider
    // gives of its own.
    readonly property var marks: Theme.glyph

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
            source.answered.connect(() => root.reanswer(source));
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
        if (held.length > 0 && category !== null)
            return answeringIn(category, letters);
        if (held.length > 0 || letters.trim() === "")
            return null;
        for (const source of sources) {
            const said = source.answer(letters);
            if (said)
                return heard(said, source, letters);
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
                return Object.assign(heard(said, source, question), { question: question });
        }
        return null;
    }

    // An answer as the launcher keeps it: who gave it, and to what words,
    // so it can be asked again while it's stepped into and say what it says
    // now. Made over the provider's own, rather than copied, so parts it
    // makes only once they're looked at (Machine.qml's fresh) still are.
    function heard(said, source, words) {
        return Object.assign(Object.create(said), { asker: source, heard: words });
    }

    // Whether an answer has parts, without making them: one whose parts are
    // made only once they're looked at says it has some.
    function parted(said) {
        return !!said && (said.parted === true || said.rows.length > 0);
    }

    // What each word asking an answer names it, as the vocabulary does, so
    // a step into an answer is named as the thing it is ("cpu" is
    // Processor), not as what was typed.
    readonly property var answerNames: {
        const out = {};
        for (const id of Object.keys(Words.things)) {
            const entry = Words.of(id);
            for (const asked of entry.answers)
                out[asked] = entry.name;
        }
        return out;
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

    // An answer with detail is one row above what was found, named by what
    // it says, its detail a step in, so a question never lists a column of
    // bare figures. A sum's, with none, is said in the field. Asked afresh,
    // the selection goes to the top; when a late answer arrives, a
    // selection already moved stays on the row it was on.
    function compose(fresh) {
        const before = matches.length > 0 && matches[0].answered ? 1 : 0;
        answer = answering(typed, fresh);
        const rows = parted(answer) ? [answered(answer)] : [];
        // A category named as a thing is, or as an answer asks, gives way to
        // it, so "sound" is Sound and "weather" the weather; its word and a
        // space still step into it.
        // At home, every category is listed, to be browsed.
        const things = new Set(root.found.filter(row => !row.category)
            .map(row => Fuzzy.plainly(row.name)));
        const found = root.found.filter(row => !row.category || atHome
            || !(things.has(Fuzzy.plainly(row.name)) || (answer && !answer.question)));
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
            gathered.filter(row => !whole.has(row.from.settings.name)), latch));
        // Searched again only to say what's there now, it keeps the order
        // it's shown in (reshow).
        matches = steadying ? Steady.kept(matches, rows.concat(below), steadyId)
            : rows.concat(below);
        // An answer only started gives way to a name that starts the same.
        const meant = answer && !(answer.question && below.length > 0 && startsAWord(below[0].name, typed));
        if (fresh || (current === -1 && !answer))
            current = meant ? (rows.length > 0 ? 0 : -1) : answer ? rows.length : 0;
        else if (current === -1 && rows.length > 0)
            current = 0;
        else if (current >= 0)
            current = Rank.pinned(matches, was, current + rows.length - before);
    }

    // An answer as a row: what it says as its name, Return copying it or
    // doing what it does, and its detail behind the right arrow, each part
    // copied the same way. Its kind says it's an answer.
    function answered(said) {
        // One whose parts wait to be looked at says its own glyph and hue.
        const first = said.parted === true ? {} : said.rows[0];
        const asked = said.question ?? Fuzzy.plainly(said.heard ?? typed);
        return {
            // Nothing to learn: an answer is always first for its question.
            key: "",
            is: "answer",
            answered: true,
            nested: true,
            // Named by what it says, not by what was typed, so nothing in
            // it is marked as matched.
            plain: true,
            name: said.headline,
            chip: asked,
            title: said.title ?? answerNames[asked] ?? root.capitalised(asked),
            glyph: said.glyph ?? first.glyph,
            graph: said.graph,
            hue: said.hue ?? first.hue,
            swatch: first.swatch,
            // Who answered, to what, and whether it moves, so a step held
            // in it is asked again as it changes.
            asker: said.asker ?? null,
            heard: said.heard ?? "",
            live: said.live ?? null,
            side: "",
            chord: [],
            verb: said.verb ?? ((said.copy ?? said.headline) === "" ? "close" : "copy"),
            now: true,
            note: said.note ?? "",
            run: said.run ?? (() => {
                const copy = said.copy ?? said.headline;
                if (copy !== "")
                    Quickshell.clipboardText = copy;
            }),
            verbs: () => said.rows
        };
    }

    // A provider has more to say: out at the top, its answer is asked
    // again; stepped in, so is whatever of its answers is held or listed.
    function reanswer(source) {
        if (!open)
            return;
        if (held.length === 0)
            compose(false);
        else
            relive(source);
    }

    // An answer row asked again, as it says now, keeping where it's held
    // and how its step is named, or null when it no longer answers.
    function again(row) {
        const said = row.asker ? row.asker.answer(row.heard) : null;
        if (!parted(said))
            return null;
        return Object.assign(answered(heard(said, row.asker, row.heard)), {
            key: row.key,
            is: row.is,
            chip: row.chip,
            title: row.title
        });
    }

    // What a source's answers say now, wherever they're shown below the
    // top: an answer stepped into, its parts and what's held in them made
    // again, or a category's own answers, listed or asked in it. What's
    // shown refreshes in place, as `refreshed` says.
    function relive(source) {
        if (asking)
            return;
        const steps = held.slice();
        const at = steps.findIndex(step => step.row.asker === source);
        const last = steps.length - 1;
        if (at >= 0) {
            const was = steps[at];
            const row = again(was.row);
            if (!row)
                return;
            steps[at] = Object.assign({}, was, {
                row: row,
                rows: Steady.kept(was.rows, row.verbs(), steadyId)
            });
            // The list it was stepped into from says it as it is now too,
            // for the way back out.
            if (at > 0)
                steps[at - 1] = Object.assign({}, steps[at - 1], {
                    rows: steps[at - 1].rows.map(one => one === was.row ? row : one)
                });
            if (!remade(steps, at + 1))
                return;
        } else if (category !== null && (steps[last].rows.some(row => row.asker === source)
            || (answer && answer.asker === source))) {
            steps[last] = Object.assign({}, steps[last], {
                rows: steps[last].rows.map(row => row.asker === source ? again(row) ?? row : row)
            });
        } else {
            return;
        }
        held = steps;
        reshow(steps.length);
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
        if (selected === headline || selected.answered)
            return "answer";
        if (selected.category)
            return "category";
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
        .concat(collections.map(named), categories)
    readonly property var collections: sources.filter(source => source.collection)

    // Each row by its key, and the rows that are parts of another (`of`
    // names it), such as Slack's theme of Slack, by the key of what they're
    // part of: listed inside it, and found from outside only by a word past
    // its name, so "slack" is Slack and "slack theme" the theme.
    readonly property var keyed: {
        const out = {};
        for (const choice of choices)
            out[choice.key] = choice;
        return out;
    }
    readonly property var partsOfRows: {
        const out = {};
        for (const choice of choices)
            if (choice.of)
                out[choice.of] = (out[choice.of] ?? []).concat([choice]);
        return out;
    }

    // Whether a part is left out of a search, every word typed only
    // naming what it's part of.
    function partHidden(choice, typedWords) {
        const whole = keyed[choice.of];
        if (!whole)
            return false;
        const named = Fuzzy.words(whole.name);
        return typedWords.every(word => named.some(one => one.startsWith(word)));
    }

    // A row's parts as they're named inside it: "Theme" in Slack.
    function partsIn(row) {
        return (partsOfRows[row.key] ?? []).map(part => Object.assign({}, part, {
            name: part.part ?? part.name
        }));
    }

    // The categories (Words.qml), each a row found by its word, whose step
    // in lists what it holds; its word and a space step in from anywhere,
    // and what's typed after searches only inside it.
    readonly property var categories: Words.categories.map(category => {
        const row = {
            key: `category:${category.id}`,
            name: category.name ?? category.id,
            verb: "open",
            plain: true,
            category: category,
            glyph: Theme.glyph.category,
            hue: "voice",
            chord: [],
            side: "",
            chip: category.id,
            ask: "what in it?",
            lead: Theme.glyph.category,
            leadHue: "voice",
            // What it holds are things with verbs of their own, as a
            // collection's are.
            nested: true,
            named: true,
            weight: 1,
            verbs: () => root.members(category)
        };
        row.prepared = Fuzzy.prepare(row.name);
        row.keywords = Fuzzy.prepare([row.name].concat(category.words ?? []).join(" "));
        return row;
    })

    // Where Joel is, as the field shows it: each step as its row named it,
    // in the rows' own case (Shortcuts, Desktop), so the path reads as the
    // rows he picked did. A step named as the one before it (the Shortcuts
    // category's Shortcuts) is said once.
    readonly property var path: held.reduce((steps, step) => {
        const said = step.row.title ?? step.row.name;
        const last = steps.length > 0 ? steps[steps.length - 1] : "";
        return said.toLowerCase() === last.toLowerCase() ? steps : steps.concat([said]);
    }, [])

    // Words with their first letter a capital, as a row's name starts.
    function capitalised(words) {
        return words === "" ? "" : words[0].toUpperCase() + words.slice(1);
    }

    // The category held, or null.
    readonly property var category: held.length > 0 ? held[held.length - 1].row.category ?? null
        : null

    // The category a word and a space step into: its own word, or one of
    // its others typed whole.
    function categoryFor(word) {
        const said = word.toLowerCase();
        return Words.categories.find(category => category.id === said
            || (category.words ?? []).includes(said)) ?? null;
    }

    // What a category holds, in its order: each thing the vocabulary names
    // as it's found anywhere, and only those this machine has.
    function members(category) {
        if (category.id === "commands")
            return commands();
        return (category.holds ?? []).map(id => resolved(id)).filter(row => row !== null);
    }

    // What a category holds, said after its name at home so scrolling
    // through them says what's there: its members' names in its order, as
    // their rows name them ("Agents, Tasks, Helpers, AI spend"), as many
    // whole as fit in `room` characters, but for one its own name already
    // says (Sound's Sound). Read from the vocabulary as home is made, so no
    // answer is asked for it.
    function holding(category, room) {
        const names = category.id === "commands"
            ? commands().map(row => row.start.trim() || row.key.replace(/^command:/, ""))
            : (category.holds ?? []).map(id => heldName(id)).filter(name => name !== "");
        const own = (category.name ?? category.id).toLowerCase();
        const cased = names.filter(name => name.toLowerCase() !== own);
        const more = ", \u2026";
        let said = "";
        for (let at = 0; at < cased.length; at++) {
            const next = said === "" ? cased[at] : `${said}, ${cased[at]}`;
            const last = at === cased.length - 1;
            if (next.length + (last ? 0 : more.length) > room)
                return said === "" ? "" : `${said}${more}`;
            said = next;
        }
        return said;
    }

    // The name of a thing a category holds, as it's shown now, or "" where
    // this machine has none or its name waits for a fact ("Sign in to
    // {network}").
    function heldName(id) {
        const known = keyed[id];
        if (known && !known.category)
            return known.name;
        const entry = Words.of(id);
        const there = sources.some(one => one.settings.name === (entry.provider ?? id));
        return there && !entry.name.includes("{") ? entry.name : "";
    }

    // A thing the vocabulary names, as a row: the row of that key, a
    // collection whole, an answer to its question, or a provider's things
    // as one row holding them. Null where this machine has none.
    function resolved(id) {
        const known = keyed[id];
        if (known && !known.category)
            return known;
        const entry = Words.of(id);
        const source = sources.find(one => one.settings.name === (entry.provider ?? id));
        if (!source)
            return null;
        if (source.collection && !entry.question)
            return whole(source, false);
        const question = entry.question ?? source.questions[0];
        if (question !== undefined) {
            const said = source.answer(question);
            return parted(said) ? Object.assign(answered(heard(said, source, question)), {
                is: `answer:${id}`,
                chip: question
            }) : null;
        }
        const things = source.things.filter(row => !(row.key.startsWith("action:")
            && standIns.includes(row.name)));
        if (things.length === 0)
            return null;
        const row = {
            key: `group:${id}`,
            name: entry.name,
            verb: "open",
            plain: true,
            glyph: Words.glyph(id),
            hue: "voice",
            chord: [],
            side: "",
            count: String(things.length),
            chip: entry.name.toLowerCase(),
            title: entry.name,
            nested: true,
            group: true,
            verbs: () => things
        };
        row.prepared = Fuzzy.prepare(row.name);
        return row;
    }

    // Every command that takes something after it, its form shown, from
    // the providers this machine has: Return or Tab puts its start in the
    // field, out at the top, so it's finished there.
    function commands() {
        return Words.commands.filter(command => sources.some(source => source.settings.name
            === command.provider)).map(command => {
            const row = {
                key: command.id,
                name: `${command.start}${command.form}`.trim(),
                form: command.form,
                start: command.start,
                verb: "type",
                glyph: Theme.glyph.command,
                hue: "action",
                chord: [],
                side: command.says ?? "",
                plain: true
            };
            row.prepared = Fuzzy.prepare(row.name);
            row.keywords = Fuzzy.prepare([row.name].concat(command.words ?? []).join(" "));
            return row;
        });
    }

    // A collection's parts the vocabulary names, before its own members:
    // Agents holds its tasks, helpers and spend, the Keyboard its
    // keyboards and shortcuts.
    function partsOf(source) {
        return Words.of(source.settings.name).parts.map(id => resolved(id))
            .filter(row => row !== null);
    }

    // Rows found by what's typed, inside a step, a category or a
    // collection's parts, in the top level's tiers (Rank.js): a name that
    // is what's typed, then one starting with it, then one with a later
    // word starting with it, and then a row found only by its hidden words
    // (hiddenWords), each tier best match first. A hidden word isn't said beside
    // the row, since it may be a name the machine knows a thing by, such as
    // tailscale0, that Joel never sees.
    function within(rows, letters) {
        const typedWords = Fuzzy.words(letters);
        const need = Fuzzy.letters(typedWords);
        const found = [];
        for (const row of rows) {
            const prepared = row.prepared ?? Fuzzy.prepare(row.name);
            const byName = (prepared.mask & need) === need
                ? Fuzzy.match(prepared, typedWords, false) : { score: -1 };
            if (byName.score >= 0) {
                found.push({
                    item: prepared,
                    score: byName.score,
                    grade: byName.grade,
                    tier: Rank.tier(row.name, letters, false),
                    choice: row
                });
                continue;
            }
            const known = hiddenWords(row);
            if (!known || (known.mask & need) !== need)
                continue;
            const byWord = Fuzzy.match(known, typedWords, false);
            if (byWord.score >= 0)
                found.push({
                    item: prepared,
                    score: byWord.score,
                    grade: byWord.grade,
                    tier: Rank.wordTier(known.text, letters),
                    choice: row
                });
        }
        return Rank.order(found, {}).map(f => f.choice);
    }

    // A row's hidden words, prepared to match: its keywords, and what the
    // vocabulary says finds the thing it is (Words.qml's keywords), by its
    // key, or the provider's things it holds, or the answer it gives.
    // Made once for each row and kept on it, since a step's rows are
    // searched at every key; a WeakMap keeping them crashed the shell once
    // its rows were collected.
    function hiddenWords(row) {
        if (row.hiddenMade !== undefined)
            return row.hiddenMade;
        const key = String(row.key ?? "");
        const id = key.startsWith("group:") ? key.slice("group:".length)
            : String(row.is ?? "").startsWith("answer:") ? row.is.slice("answer:".length) : key;
        const said = id !== "" && Words.things[id] ? Words.keywords(id) : "";
        const own = row.keywords ? row.keywords.text ?? "" : "";
        const words = [own, said].filter(part => part !== "").join(" ");
        const made = words === "" ? null : Fuzzy.prepare(words);
        row.hiddenMade = made;
        return made;
    }

    // What's typed in a category: what it holds, and what's held in the
    // things it holds as one row (Programs' programs), found by their
    // words, then each of its collections' best match, as at the top. A
    // row named what's typed, whole, comes first wherever it was found, so
    // "workspaces" in Shortcuts is its Workspaces group, before any chord
    // or act that only uses the word.
    function inCategory(letters) {
        const pool = offered.reduce((all, row) => all.concat([row], row.group ? row.verbs() : []),
            []);
        const inside = offered.filter(row => row.from).map(row => row.from);
        return namedFirst(within(pool, letters).concat(gathering(letters, inside)), letters);
    }

    // Rows named what's typed, whole, before the rest, each in its order.
    function namedFirst(rows, letters) {
        const said = Fuzzy.plainly(letters);
        const named = row => Fuzzy.plainly(row.name) === said;
        return rows.filter(named).concat(rows.filter(row => !named(row)));
    }

    // What a category's own answers make of what's typed in it: in
    // computer, "battery". Commands answer nothing there, since what's
    // typed finds a command, which finishes at the top.
    function answeringIn(category, letters) {
        if (letters.trim() === "" || category.id === "commands")
            return null;
        const names = (category.holds ?? []).map(id => Words.of(id).provider ?? id);
        for (const source of sources.filter(one => names.includes(one.settings.name))) {
            const said = source.answer(letters);
            if (said)
                return heard(said, source, letters);
        }
        return null;
    }

    // Whether the category's word and what's typed after it ask something
    // whole ("time in tokyo", "time left", "weather tokyo"): a question,
    // not a search inside it, so the field gives it back whole.
    function askedWhole(letters) {
        if (held.length !== 1 || Fuzzy.words(letters).length === 0)
            return "";
        const whole = `${held[0].typed} ${letters}`;
        return sources.some(source => source.answer(whole)) ? whole : "";
    }

    // Steps into a category from a word and a space, the word put back on
    // the way out.
    function intoCategory(category, word) {
        const row = categories.find(one => one.category === category);
        held = [Object.assign(here(), { typed: word, row: row, rows: members(category) })];
        restart("");
    }

    // Puts a command's start in the field, out at the top, where it's
    // finished.
    function fill(start) {
        held = [];
        restart(start);
    }

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

    function gathering(letters, among) {
        if (letters.replace(/\s+/g, "").length < 2)
            return [];
        const rows = [];
        for (const source of among ?? collections) {
            // One opened only by its own chord adds no row to a search, but
            // does inside a category that holds it.
            if (source.collection.searched === false && !among)
                continue;
            const said = source.gather(letters);
            // Inside a category, its best is one named what's typed before
            // one picked for it.
            const learned = learnedFirst(said.rows, letters);
            const found = among ? namedFirst(learned, letters) : learned;
            if (found.length === 0)
                continue;
            const best = found[0];
            const noun = source.collection.noun;
            rows.push(Object.assign({}, best, {
                side: found.length > 1 ? `${found.length - 1} more ${noun}` : noun,
                chip: noun,
                title: source.collection.title,
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
        if (row && row.answered)
            return "answer";
        if (!row || row.opens || row.plain)
            return "";
        return kinds[row.what ?? ""] ?? "";
    }

    // The one word the foot gives Return: the row's own, or else an act's
    // first word ("forget", "disconnect"), which its name starts with, and
    // which way for a turn ("turn off"), or "open" for a thing.
    function verbOf(row) {
        if (!row)
            return "";
        if ((row.verb ?? "") !== "")
            return oneWord(row.verb);
        return kindOf(row) !== "" ? "open" : oneWord(Rank.verbIn(row.name ?? "") || "open");
    }

    // A verb as the foot says it, one word (Joel, 2026-10-24: "Switch to
    // it" was too many): a turn by the way it goes ("off"), anything else
    // by its first word.
    function oneWord(verb) {
        const said = verb.split(" ");
        return said[0] === "turn" && said.length > 1 ? said[said.length - 1] : said[0];
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
        // What's found at a shorter start isn't known, so any row picked
        // there counts against this one, and it's said only when sure.
        const there = Rank.maybeThere(foundHere);
        for (let length = 1; length < letters.length; length++) {
            const start = letters.slice(0, length);
            if (picks.latched(start, there, namedAs[start] ?? []) === row.key)
                return start;
        }
        return "";
    }

    // What's latched for what's typed, worked out once a search, and the
    // rows it found, by key, each with the fewest letters it's learned at
    // (Rank.js's leastOf), so a row that's gone counts for nothing.
    property string latch: ""
    property var foundHere: ({})

    // The keys of the rows named each name, so a name typed whole is first
    // over another row latched there (Rank.js's latched).
    readonly property var namedAs: {
        const out = {};
        for (const choice of choices) {
            const name = Rank.plainly(choice.name);
            (out[name] ?? (out[name] = [])).push(choice.key);
        }
        return out;
    }

    // Every row there is now, by key, each with the fewest letters it's
    // learned at, for forgetting the picks of those that are gone: what the
    // main search finds from, and what each collection holds. Made only
    // when something's picked.
    function knowing() {
        const out = {};
        const add = row => {
            if (row && row.key)
                out[row.key] = Rank.leastOf(row);
        };
        choices.forEach(add);
        for (const source of collections)
            source.everything().forEach(add);
        return out;
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
    // it, so the chords are learned by seeing them where they're used, and
    // the categories, each saying what it holds, so scrolling down reminds
    // Joel what there is.
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
        const places = kept.map(source => whole(source, false)).concat(browsed());
        if (places.length > 0 && rows.length > 0)
            places[0] = Object.assign({}, places[0], { gap: true });
        return rows.concat(places);
    }

    // The categories as home lists them, in the vocabulary's order, each
    // with what it holds after its name, as much as the row has room for.
    // One led by a collection the shelf already shows (Clipboard,
    // Notifications) is left to the shelf's row, chord and all, so home
    // never says the same thing twice.
    function browsed() {
        const glyphs = Theme.space.l + Theme.space.xxl;
        const room = Math.floor((list.width - glyphs - Theme.space.xxl - Theme.space.m
            - Theme.space.l) / Math.max(1, measure.advanceWidth("0")));
        const shelved = row => {
            const first = (row.category.holds ?? [])[0];
            return first !== undefined && shelf.includes(Words.of(first).provider ?? first);
        };
        return categories.filter(row => !shelved(row)).map(row => Object.assign({}, row, {
            count: holding(row.category, room > 0 ? room - row.name.length : Infinity)
        }));
    }

    // Where Tab at home goes: the first category, so browsing them takes
    // one key.
    function browse() {
        const first = matches.findIndex(row => !!row.category);
        if (first >= 0)
            current = first;
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
            title: collection.title,
            ask: collection.ask,
            lead: collection.glyph,
            leadHue: "voice",
            verbs: () => source.everything()
        };
    }

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
            key: row ? steadyId(row) : "",
            scroll: list.contentY
        };
    }

    // Selects what was selected before, by what it is when it's still
    // there, so of two windows titled alike it's the same one, since what
    // was searched again may have moved it, or else by where it was, and
    // scrolls the list back to where it stood.
    function reselect(left) {
        const byKey = left.key === "" ? -1
            : matches.findIndex(row => steadyId(row) === left.key);
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
        const things = held.length === 0 || collecting;
        const own = things ? (row.verbs ? row.verbs() : null) : row.targets ? row.targets() : null;
        // A thing's parts come after what can be done to it.
        const parts = things ? partsIn(row) : [];
        const into = parts.length > 0 ? (own ?? []).concat(parts) : own;
        if (!into || into.length === 0)
            return false;
        held = held.concat([Object.assign(here(), { row: row, rows: into })]);
        if (row.from)
            row.from.opened();
        // What was typed found the row, so it goes: inside, everything the row
        // holds shows, and the left arrow puts it back on the way out.
        restart("");
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
    // and so is anything stepped into below it, each step's rows asked for
    // again, since they can say how a thing stands, as a Wi-Fi network's say
    // its access points; and a category holding it has its members made
    // again, so Agents in "ai" says how they stand. What's shown refreshes
    // in place (Steady.js): the rows keep the order they're shown in, new
    // ones come at the end, the selection stays on its row and the list
    // where it was scrolled, so nothing moves under the cursor. Only what
    // Joel types orders by rank.
    function refreshed(source) {
        if (!open || asking)
            return;
        const from = held.findIndex(step => step.row.from === source);
        const ofSource = step => !!step.row.category && holdsFrom(step.row.category, source);
        // A thing of its own stepped into from the top, as a program is, is
        // made again too, with everything it holds, so its Media and Sound
        // say how its player and its streams stand now.
        const top = from < 0 && held.length > 0 && !held[0].row.from && !held[0].row.category
            && source.things.some(row => row.key === held[0].row.key);
        if (from < 0 && !top && !held.some(ofSource))
            return;
        const steps = held.map(step => ofSource(step) ? Object.assign({}, step, {
            rows: Steady.kept(step.rows, members(step.row.category), steadyId)
        }) : step);
        const above = top ? source.things : from < 0 ? [] : source.everything();
        if (!remade(steps, top ? 0 : from < 0 ? held.length : from + 1, above))
            return;
        held = steps;
        reshow(steps.length);
    }

    // Each step from `from` on made again from the rows the one above it
    // offers now (`above`, or that step's own rows), as enter made it:
    // a thing at the top, a collection's things or a nested row's, asked
    // for their verbs and then their parts, anything else for its targets.
    // Says whether every step was still there.
    function remade(steps, from, above) {
        let rows = above ?? (from > 0 && from <= steps.length ? steps[from - 1].rows : []);
        for (let at = from; at < steps.length; at++) {
            const step = steps[at];
            const thing = rows.find(row => sameRow(row, step.row));
            const things = at === 0 || collects(steps[at - 1]);
            const own = !thing ? null : things ? (thing.verbs ? thing.verbs() : null)
                : thing.targets ? thing.targets() : null;
            const parts = thing && things ? partsIn(thing) : [];
            const into = parts.length > 0 ? (own ?? []).concat(parts) : own;
            if (!into)
                return false;
            steps[at] = Object.assign({}, step, {
                row: thing,
                rows: Steady.kept(step.rows, into, steadyId)
            });
            rows = into;
        }
        return true;
    }

    // What's shown searched again under what's held now, `count` steps,
    // and kept in place: the rows in the order they're shown, the
    // selection on its row and the list where it was scrolled.
    // The search is told it's only saying what's there now (steadying), so
    // it keeps the rows in their order itself and leaves the selection and
    // the list alone, rather than starting them afresh to be put back: the
    // rows are made once a refresh, not twice.
    function reshow(count) {
        const was = current >= 0 && selected ? steadyId(selected) : "";
        const at = current;
        const scroll = list.contentY;
        steadying = true;
        search(typed);
        steadying = false;
        // What's typed asked something whole, out at the top, now it can.
        if (held.length !== count)
            return;
        const lowest = answer && !parted(answer) ? -1 : 0;
        const now = at === -1 && answer ? -1 : Steady.selection(matches, {
            row: was,
            at: at,
            lowest: lowest
        }, steadyId);
        if (current !== now)
            current = now;
        if (list.contentY !== scroll)
            list.contentY = scroll;
        if (current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
    }

    // Whether the search under way only says again what's there, as a
    // refresh does (reshow).
    property bool steadying: false

    // What a row is, to hold it in place as it changes: what it says it
    // is, or for a part with no key, as an answer's figures have none, its
    // name, so the Cores stay the Cores as their loads move.
    function steadyId(row) {
        return Rank.identity(row) || `named:${row.name ?? ""}`;
    }

    // Whether a row offered now is the one held: by its key, or else by
    // what it says it is, or a part with neither by its name.
    function sameRow(row, was) {
        if ((was.key ?? "") !== "")
            return row.key === was.key;
        return (was.is ?? "") !== "" ? row.is === was.is : row.name === was.name;
    }

    // Whether a category holds something of a source's, by the provider
    // each thing it holds comes from, so its members are made again only
    // when one of theirs changed.
    function holdsFrom(category, source) {
        const name = source.settings.name;
        return (category.holds ?? []).some(id => (Words.of(id).provider ?? id) === name);
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
        steadying = false;
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
        // Inside a step, the row selected as what's typed is taken away,
        // to stay on.
        const cleared = letters === "" && typed !== "" && held.length > 0 && selected
            ? steadyId(selected) : "";
        typed = letters;
        words = Fuzzy.words(letters);
        if (asking) {
            matches = [];
            answer = null;
            current = 0;
            return;
        }
        // A category's word and a space step into it.
        const chipped = held.length === 0 ? /^(\S+) $/.exec(letters) : null;
        const named = chipped ? categoryFor(chipped[1]) : null;
        if (named) {
            intoCategory(named, chipped[1]);
            return;
        }
        const whole = category !== null ? askedWhole(letters) : "";
        if (whole !== "") {
            held = [];
            restart(whole);
            return;
        }
        if (category !== null) {
            // What's typed searches only inside it, its answers first.
            gathered = [];
            root.found = words.length === 0 ? offered.slice() : inCategory(letters);
            compose(!steadying);
            if (!steadying)
                list.positionViewAtBeginning();
            return;
        }
        if (held.length > 0) {
            const from = held[held.length - 1].row.from;
            // A collection searches itself, all of it until something's
            // typed, and then its parts: Agents its agents, then their
            // tasks, helpers and spend. A part named leads what's found.
            let found = [];
            if (from)
                found = words.length === 0 ? from.everything().concat(partsOf(from))
                    : within(partsOf(from), letters)
                        .concat(learnedFirst(from.search(letters).rows, letters));
            // A step's rows, all of them in their order until something's
            // typed, then those it finds, best first, and with them the
            // choices of a part that offers them (a display's Scale), so
            // what's typed reaches them without a step in: "scale 150".
            // Found by name or by hidden word, in the top level's tiers.
            else if (words.length === 0)
                found = offered.slice();
            else
                found = within(offered.reduce((all, row) => all.concat([row],
                    row.offers && row.verbs ? row.verbs() : []), []), letters);
            answer = null;
            // Searched again only to say what's there now, it keeps the
            // order it's shown in, and the selection where it is (reshow).
            if (steadying) {
                matches = Steady.kept(matches, found, steadyId);
                return;
            }
            matches = found;
            // What's typed taken away whole leaves the selection on the row
            // it was on, now among them all.
            const kept = cleared ? matches.findIndex(row => steadyId(row) === cleared) : -1;
            current = Math.max(0, kept);
            if (kept >= 0)
                list.positionViewAtIndex(kept, ListView.Contain);
            else
                list.positionViewAtBeginning();
            return;
        }
        gathered = gathering(letters);
        latch = "";
        foundHere = {};
        if (words.length === 0) {
            candidates = [];
            root.found = home();
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            // What the next letter narrows from: everything found, and a
            // collection held back only for being too short yet.
            const early = [];
            for (const choice of narrowing ? candidates : choices) {
                // A part whose whole is all that's named is found in it,
                // and kept for a word past the whole's name.
                if (choice.of && partHidden(choice, words)) {
                    early.push(choice);
                    continue;
                }
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
                // A collection's own words are found by their start from
                // three letters, and below that only whole, so "pi" is pi's
                // sessions and not the Clipboard's pictures.
                const tooShort = choice.named && Fuzzy.plainly(letters).length < 3
                    && !Rank.says(choice.keywords.text ?? "", letters);
                if (byWord.score >= 0 && tooShort)
                    early.push(choice);
                else if (byWord.score >= 0)
                    found.push({
                        item: choice.prepared,
                        score: byWord.score * choice.weight / (choice.named ? 1 : 4),
                        grade: byWord.grade,
                        tier: choice.named ? 1 : Rank.wordTier(choice.keywords.text ?? "", letters),
                        choice: choice,
                        byWord: true
                    });
            }
            candidates = found.map(f => f.choice).concat(early);
            const learned = picks.weights(letters);
            const here = {};
            for (const f of found)
                here[f.choice.key] = Rank.leastOf(f.choice);
            for (const row of gathered)
                for (const member of row.verbs())
                    here[member.key] = Rank.leastOf(member);
            foundHere = here;
            latch = picks.latched(letters, Rank.foundNow(here),
                namedAs[Rank.plainly(letters)] ?? []);
            const ordered = Rank.order(found, {
                latched: latch,
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
        // An answer's row says the answer; the field finishes its question.
        if (choice.answered)
            return answer && answer.question
                ? answer.question.slice(Fuzzy.plainly(typed).length) : "";
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
        // A command's form, what's typed after its start, is muted, so
        // what to type reads apart from what to put there.
        if ((choice.form ?? "") !== "")
            out = out.replace(/\u27e8[^\u27e9]*\u27e9/g, said => `<font color="${Theme.muted}">${said}</font>`);
        return out;
    }

    function run(choice, below) {
        if (!choice) {
            open = false;
            return;
        }
        // A command puts its start in the field, to be finished there.
        if (choice.start !== undefined) {
            fill(choice.start);
            return;
        }
        // A category, or a row holding a provider's things, steps in.
        if ((choice.category || choice.group) && enter(choice))
            return;
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
        if (choice.confirm && (held.length === 0 || collecting) && enter(choice))
            return;
        // One with nothing to do of its own, only more inside, steps in.
        if (!choice.run && enter(choice))
            return;
        // One whose outcome takes a while keeps the launcher, which says how
        // it's going.
        if (choice.stays) {
            choice.run();
            return;
        }
        // One that opens a collection steps into it, the launcher kept open.
        if (choice.opens) {
            picks.picked(typed, choice.key, knowing());
            into(choice.opens);
            return;
        }
        // The clipboard takes text only from a program holding the keys.
        if (choice.now)
            choice.run();
        open = false;
        // Acting on a thing says it was what was meant by what was typed;
        // in a collection or a category, the one picked from it was.
        const first = held.length > 0 ? held[0] : { row: choice, typed: typed };
        const picking = !!first.row.from || !!first.row.category;
        const meant = picking ? choice : first.row;
        // One that can't be taken back is latched only once its first word's
        // been typed (Rank.js's leastOf), so a slip of two letters never
        // powers off.
        picks.picked(picking ? typed : first.typed, meant.key, knowing());
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
            path: root.path,
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
                // The whole words drawn below the rows for it, if any.
                reads: row.reads ?? "",
                settled: !(row === root.headline || row.answered)
                    || !(root.answer && root.answer.asking)
            },
            kind: root.kind,
            footer: foot.said,
            glance: standing.said
        });
    }

    // Said each time it changes, while reports are on, so the test hears
    // every step rather than asking over and over.
    readonly property string shown: Options.reports ? showing() : ""

    onShownChanged: Report.say(`showing ${shown}`)

    // The window the launcher opened over, as niri describes it, or null on
    // an empty workspace: what Return pastes into (Pasting.qml), and whose
    // program's keys Shortcuts leads with (launcher/KeyMap.qml).
    function cameFrom() {
        return Pasting.under();
    }

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
                title: source.collection.title,
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
    // With no screen left, Qt lists a stand-in of its own, which has no name
    // and can hold no surface, so it's never one.
    readonly property var focusedScreen: {
        Niri.revision;
        const workspace = Niri.workspace();
        const real = Quickshell.screens.filter(screen => screen.name !== "");
        return real.find(screen => workspace && screen.name === workspace.output)
            ?? real[0] ?? null;
    }
    property var screenHere: focusedScreen

    onFocusedScreenChanged: {
        if (!open)
            screenHere = focusedScreen;
    }

    // A screen coming or going (a dock, the lid, a monitor) can leave the
    // surface nowhere. With no screen left, Qt moves the window onto a
    // stand-in that can hold no layer surface; when a screen comes back, Qt
    // moves the window onto it by itself, so Quickshell sees no change of
    // screen and never makes the surface again, and Super+Space finds
    // nothing. So once the screens have changed, the launcher closes and its
    // surface is made again on the screen the keys are on, as the margin's
    // are made anew for each screen. With no screen at all, there's nowhere
    // to make it: niri would refuse it and Quickshell warn, so it waits
    // unmade for the next screen, which changes the screens again.
    Connections {
        target: Quickshell

        function onScreensChanged() {
            Qt.callLater(root.remake);
        }
    }

    function remake() {
        open = false;
        screenHere = focusedScreen;
        surface.visible = false;
        surface.visible = screenHere !== null;
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
        id: surface
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
                        glyph: secret ? Theme.glyph.secret
                            : thing ? thing.lead ?? thing.glyph : Theme.glyph.launcher
                        hue: secret ? Theme.machine
                            : thing ? Theme[thing.leadHue ?? thing.hue] : Theme.muted
                        glyphWidth: Theme.space.xxl
                        chips: root.path.map(step => ({ text: step }))
                        hint: root.held.length === 0 ? "what next?"
                            : root.held[root.held.length - 1].row.ask ?? "what to do?"
                        ghost: root.completion
                        // What the selection has to say, since the foot
                        // always says how things stand; else where the
                        // selection is, once there's more than fits. The
                        // row says what it is, so this doesn't.
                        aside: foot.said !== "" ? foot.said
                            : root.matches.length > root.rows && root.current >= 0
                                ? `${root.current + 1} of ${root.matches.length}` : ""
                        // What's typed narrows a list, so the left arrow
                        // takes it away whole before it steps back out.
                        leftClears: true
                        onTextChanged: root.search(text)
                        onAccepted: below => root.asking ? root.give()
                            : root.run(root.selected, below)
                        // Tab on an answer puts the answer in place of the
                        // question, so a sum can be carried on; one still
                        // being asked for leaves the question as it is.
                        onCompleted: {
                            // At home, Tab goes to the categories, to
                            // browse what there is.
                            if (root.atHome)
                                root.browse();
                            // A command puts its start in the field.
                            else if (root.current !== -1 && root.selected?.start !== undefined)
                                root.fill(root.selected.start);
                            // A question only started is finished first.
                            else if (root.question !== ""
                                && (root.current === -1 || root.selected?.answered))
                                text = root.question;
                            // A row that holds more is stepped into, so a
                            // few letters and Tab reach inside a thing from
                            // anywhere, as the right arrow would, and so
                            // does Tab alone inside a step.
                            else if ((root.typed !== "" || root.held.length > 0)
                                && root.current !== -1
                                && root.enter(root.matches[root.current]))
                                return;
                            else if (root.current !== -1 || !root.answer)
                                text = text + root.completion;
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
                        readonly property int highest: root.answer && !root.parted(root.answer)
                            ? -1 : 0
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
                            // How many characters the widest of what graphed rows
                            // say at their right runs to, as Choice says it, so
                            // their graphs stand in one column.
                            readonly property int figures: Math.max(0, ...root.matches
                                .filter(row => !!row.graph).map(row => ((root.held.length === 0
                                    ? root.kindOf(row) : "") || ((row.chord ?? []).length > 0 ? ""
                                    : row.side ?? "")).length))
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            model: root.matches

                            delegate: Item {
                                id: entry

                                required property var modelData
                                required property int index
                                // What it's inside, whose glyph an act of its
                                // own without one wears.
                                readonly property var thing: root.held.length > 0
                                    ? root.held[root.held.length - 1].row : null
                                // Its own glyph, or its thing's, for an act.
                                readonly property bool own: (modelData.glyph ?? "") !== ""
                                // Holding more (an answer's parts, a row of rows, a
                                // collection, a part whose verb is "more") before
                                // doing, and a figure only once it does nothing
                                // else (Provider.qml's copyRow).
                                readonly property bool holds: !!(modelData.opens
                                    || modelData.nested || modelData.verb === "more")
                                readonly property string sort: holds ? "holds"
                                    : modelData.figure === true && !modelData.verbs ? "figure"
                                    : "act"

                                width: list.width
                                height: Theme.space.xxxl + (modelData.gap ? list.apart : 0)

                                Choice {
                                    anchors.bottom: parent.bottom
                                    selected: entry.index === root.current
                                    glyph: entry.own || entry.sort !== "act"
                                        ? entry.modelData.glyph ?? "" : entry.thing?.glyph ?? ""
                                    // A colour's swatch wears the colour itself.
                                    hue: entry.modelData.swatch
                                        ?? Theme[(entry.own ? entry.modelData.hue
                                            : entry.thing?.hue) ?? ""] ?? Theme.text
                                    swatch: !!entry.modelData.swatch
                                    sort: entry.sort
                                    graph: entry.modelData.graph ?? null
                                    figures: list.figures
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
                            glyph: root.typed === "" ? Theme.glyph.greeting
                                : Theme.glyph.nothingFound
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

                    // A row's whole words, while it's selected, when one line
                    // cuts them short: a past notice's summary and body, wrapped
                    // below the rows as a picture is shown, at most six lines,
                    // aligned with the footer's words. Joel couldn't tell what
                    // a notice had said from its one line.
                    Text {
                        id: reading

                        readonly property string said: root.selected
                            ? root.selected.reads ?? "" : ""

                        x: Theme.space.l
                        width: parent.width - 2 * Theme.space.l
                        topPadding: Theme.space.m
                        // The column skips it while it's hidden. A height bound
                        // to its own layout would loop, since elided text lays
                        // out to the height it's given.
                        visible: said !== ""
                        text: said
                        textFormat: Text.PlainText
                        wrapMode: Text.Wrap
                        maximumLineCount: 6
                        elide: Text.ElideRight
                        color: Theme.text
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.large
                    }

                    // The foot, one line, the one place that teaches, so the
                    // rows stay clean. On the right, what Return does here,
                    // Shift+Return where it can, and where the arrows step.
                    // How things stand, always, on the left, and the keys on
                    // the right. What the selection has to say (why it's
                    // first when learning put it there, where a held thing
                    // is, where an answer came from) is said at the field's
                    // right end instead, so the glance never gives way to
                    // it: Joel often didn't see the battery or the network.
                    Item {
                        id: foot

                        readonly property var choice: root.selected ?? root.asking
                        // The room the left has, short of the keys.
                        readonly property real room: keys.x - Theme.space.xl - Theme.space.l

                        // An answer's footer says where it came from.
                        readonly property string said: {
                            if (!choice)
                                return "";
                            if (choice.state)
                                return choice.state();
                            if ((choice.note ?? "") !== "")
                                return choice.note;
                            if (choice.plain && root.answer)
                                return root.answer.note ?? "";
                            if (root.held.length > 0)
                                return root.held[0].row.context ?? "";
                            // What's typed is longer than it need be,
                            // worked out once, since each start of what's
                            // typed asks every pick.
                            const shorter = root.enough(choice);
                            if (shorter !== "")
                                return `${shorter} is enough`;
                            // A thing's row says its kind where a chord
                            // would be, so its chord is taught here.
                            return root.kindOf(choice) !== "" && (choice.chord ?? []).length > 0
                                ? `next time ${root.chordText(choice.chord)}` : "";
                        }

                        width: parent.width
                        height: Theme.space.xxl + Theme.space.s

                        // How things stand, the quietest thing here: the
                        // facts the margin's pieces hold that are always
                        // true (the time, the date, the battery, the
                        // network, the volume, the day's AI spend, the
                        // agents), each its glyph and the least that
                        // tells Joel something, as a key hint is its key
                        // and what it does, at the hints' size and step,
                        // all muted but a fact that wants him, which
                        // wears its colour. It's the same whatever's
                        // typed, held or asked, so it's always where he
                        // looks for it. As room runs short, each says its
                        // least, then by collapse rank, lowest first, its
                        // glyph alone, and only then do they give way, in
                        // that order (launcher/Standing.js). Two
                        // facts side by side under one glyph (the day's
                        // AI spend and the agents, both AI's) wear it
                        // once.
                        Row {
                            id: standing

                            readonly property var facts: Facts.all
                                .filter(fact => fact.entry.tier !== "when" && fact.piece
                                    && fact.piece.present && fact.piece.value !== "")
                                .map(fact => ({
                                    entry: fact.entry,
                                    sign: fact.piece.sign !== "" ? Theme.glyph[fact.piece.sign] ?? "" : "",
                                    glance: fact.piece.glance ?? "",
                                    least: fact.piece.least ?? "",
                                    piece: fact.piece
                                }))
                                // The network's glyph alone says all is well.
                                .filter(fact => fact.sign !== "" || fact.glance !== "")

                            readonly property var kept: {
                                const wide = facts => facts.reduce((sum, fact) => sum + measure.advanceWidth(fact.said), 0)
                                    + Math.max(0, facts.length - 1) * spacing;
                                return Standing.fitted(facts, told => wide(told) <= foot.room);
                            }
                            // As it's drawn, for a test.
                            readonly property string said: kept.map(fact => fact.said).join("  ")

                            x: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: keys.spacing

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
                                // The left arrow takes what's typed first, then goes back.
                                if (root.typed !== "")
                                    keys.push(["\u2190", "clear"]);
                                else if (root.held.length > 0)
                                    keys.push(["\u2190", "back"]);
                                if (root.atHome && root.matches.some(row => !!row.category))
                                    keys.push(["\u21e5", "browse"]);
                                return keys;
                            }
                        }
                    }
                }
            }
        }
    }
}
