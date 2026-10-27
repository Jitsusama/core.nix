// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return runs it, or Shift+Return opens it below the
// focused window. Tab takes the completion the command line offers, and does
// nothing at home, where the arrows already reach every category. Each
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
import "launcher/Catalogue.js" as Catalogue
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
            const question = source.questions.find(asked => asked.startsWith(text)
                && !oldWords[asked]);
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
    // The names things had, as they're typed, each as it was written, so
    // a name had asks only whole ("sto" asks nothing of the disk) and its
    // answer says what it was once called.
    readonly property var oldWords: {
        const out = {};
        for (const entry of Object.values(Words.things))
            for (const old of entry.once ?? [])
                out[old.toLowerCase()] = old;
        return out;
    }

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
        const said = answering(typed, fresh);
        // An answer only there when nothing else is, as asking a model by a
        // question's first word is, gives way to anything found.
        answer = said && said.fallback
            && (root.found.some(row => !row.category) || gathered.length > 0) ? null : said;
        // An answer still being asked for holds its row from the keystroke,
        // saying so, and one that held a row keeps it until a key, even if
        // it comes to nothing: added only once it came, its row pushed
        // every row below it down a moment after typing, twice for the
        // weather (Joel, 2026-10-25: "it needs to be fucking stable").
        const holds = !!answer && (answer.asking === true || (!fresh && before === 1));
        const rows = parted(answer) || holds ? [answered(answer)] : [];
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
        // A program's own keys named as it is are the program's, a step in.
        const below = Rank.keysHeld(Rank.once(Rank.placed(found.filter(once),
            gathered.filter(row => !whole.has(row.from.settings.name)), latch)));
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
        const first = said.parted === true ? {} : said.rows[0] ?? {};
        const asked = said.question ?? Fuzzy.plainly(said.heard ?? typed);
        return {
            // Nothing to learn: an answer is always first for its question.
            key: "",
            is: "answer",
            answered: true,
            nested: parted(said),
            // Named by what it says, not by what was typed, so nothing in
            // it is marked as matched.
            plain: true,
            name: said.headline,
            chip: asked,
            title: said.title ?? answerNames[asked] ?? root.capitalised(asked),
            // Asked by a name it had, it says so, as a row found by one does.
            once: oldWords[asked] ? [oldWords[asked]] : [],
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
            // One whose doing takes a while keeps the launcher open, as
            // asking a question does, its whole words below the rows.
            stays: said.stays ?? false,
            reads: said.reads ?? "",
            note: said.note ?? "",
            run: said.run ?? (() => {
                const copy = said.copy ?? said.headline;
                if (copy !== "")
                    Quickshell.clipboardText = copy;
            }),
            // One held while it's still asked for has nothing in it yet.
            verbs: parted(said) ? () => said.rows : undefined
        };
    }

    // A provider has more to say: out at the top, its answer is asked
    // again; stepped in, so is whatever of its answers is held or listed.
    // With nothing typed nothing is asked, so home isn't made again: it
    // was, each time the machine's queue changed, and a new list scrolled
    // back to the top with the selection out of sight below it (Joel,
    // 2026-10-25: "jumpy ... at home arrowing around"). An answer asked
    // again keeps the list where it was scrolled, as a refresh does.
    function reanswer(source) {
        // At home nothing is asked again, even of a place that's one answer
        // (Disk), so home is never made again under him.
        if (!open || (held.length === 0 && typed.trim() === ""))
            return;
        Report.say(`asked again by ${source.settings.name}`);
        if (held.length > 0) {
            relive(source);
            return;
        }
        const scroll = list.contentY;
        compose(false);
        if (list.contentY !== scroll)
            list.contentY = scroll;
        if (current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
    }

    // An answer row asked again, as it says now, keeping where it's held
    // and how its step is named, or null when it no longer answers.
    function again(row) {
        const said = row.asker ? row.asker.answer(row.heard) : null;
        if (!parted(said))
            return null;
        return inPlace(Object.assign(answered(heard(said, row.asker, row.heard)), {
            key: row.key,
            is: row.is,
            chip: row.chip,
            title: row.title
        }), row.placed);
    }

    // An answer a category holds, named as the vocabulary names it, what it
    // says beside: a list by name keeps it in one place however its figure
    // changes, where "Sunday 25 October 2026" moved every day. Out at the
    // top, an answer is named by what it says, as it's what was asked.
    function inPlace(row, name) {
        if (!name)
            return row;
        return Object.assign(row, { name: name, side: row.name, placed: name });
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
        stays: answer.stays ?? false,
        reads: answer.reads ?? "",
        note: answer.note ?? "",
        // Asked by a name it had, it says so, as a row found by one does.
        once: oldWords[Fuzzy.plainly(typed)] ? [oldWords[Fuzzy.plainly(typed)]] : [],
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
        .concat(collections.map(named), categories)
        .map(withSynonyms)
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

    // A spot: a part or a figure a level inside a thing a category holds,
    // a headset's Sample rate or a screen's Colour depth, found from the
    // root by its name, saying where it lives, and Return goes there, the
    // crumbs filled and the spot selected, so ← goes back up through each
    // (Joel, 2026-10-25: "being able to drill right down by typing in the
    // details from a root view that would put me in the hierarchy"). Only
    // what a thing reads or holds is a spot; what it does, such as
    // Disconnect, is said once for each thing and would be found many
    // times over. Made once an opening, by the first search of three
    // letters or more, of the collections whose things are devices, few
    // and read whole, so a search after it never reads them again.
    readonly property var spotCollections: ["sound", "bluetooth", "displays", "wifi",
        "keyboard", "cameras"]
    readonly property int spotsFewest: 3
    property var spots: []
    property bool spotsMade: false

    // An answer's parts are spots too, as Battery's Health is, from the
    // answers a place holds and those home is, such as Disk: asked already
    // as their places are made, so finding them asks nothing more.
    function spotsFrom() {
        let out = [];
        for (const shown of categories) {
            const held = members(shown.category);
            const collections = held.filter(row => row.from
                && spotCollections.includes(row.opens));
            for (const collection of collections)
                for (const thing of collection.verbs())
                    for (const part of (thing.verbs ? thing.verbs() ?? [] : []).concat(partsIn(thing)))
                        if (isSpot(part))
                            out.push(spotOf(part, [shown, collection, thing]));
            for (const said of held.filter(row => row.answered))
                out = out.concat(answerSpots(said, [shown, said]));
        }
        for (const id of Words.home.filter(one => !placeNamed(one))) {
            const place = answerPlace(id);
            if (place && place.answered)
                out = out.concat(answerSpots(place, [place]));
        }
        return out;
    }

    // What's asked whole at the top, by any thing's words: a part named
    // so, as the processor's Cores, is found as that answer, never as a
    // second row beside it.
    readonly property var answerWords: {
        const out = {};
        for (const entry of Object.values(Words.things))
            for (const word of entry.answers ?? [])
                out[Fuzzy.plainly(word)] = true;
        return out;
    }

    function answerSpots(said, trail) {
        return (said.verbs ? said.verbs() ?? [] : [])
            .filter(part => isSpot(part) && !answerWords[Fuzzy.plainly(part.name)])
            .map(part => spotOf(part, trail));
    }

    function isSpot(part) {
        return !!part && (part.name ?? "") !== ""
            && (part.figure === true || !!part.targets || !!part.verbs || !!part.nested);
    }

    function spotOf(part, trail) {
        // A place inside another says it: Battery's Health is in Machine,
        // then Power, then Battery.
        const above = trail[0].category
            ? placesAbove(trail[0].category).map(place => place.name ?? place.id) : [];
        const spot = {
            key: `spot:${trail.map(row => steadyId(row)).join("/")}/${steadyId(part)}`,
            name: part.name,
            glyph: part.glyph,
            hue: part.hue,
            graph: part.graph,
            side: above.concat(trail.map(row => row.name)).join(" \u203a "),
            chord: [],
            verb: "go",
            // Where it is, and so where Return goes, by what each step is.
            trail: trail.map(row => steadyId(row)),
            spot: steadyId(part),
            stays: true,
            prepared: Fuzzy.prepare(part.name),
            once: oldNames(part)
        };
        spot.olds = spot.once.length > 0 ? Fuzzy.prepare(spot.once.join(" ")) : null;
        spot.run = () => root.goTo(spot);
        return spot;
    }

    // The spots what's typed names, best first, once it's three letters or
    // more, but for one a row found already says: a thing found at the top
    // is its own row.
    function spotted(letters) {
        if (Fuzzy.plainly(letters).length < spotsFewest)
            return [];
        if (!spotsMade) {
            spots = spotsFrom();
            spotsMade = true;
        }
        const found = [];
        for (const spot of spots) {
            // By its name, or else by a name it had.
            const byName = Fuzzy.match(spot.prepared, words, false);
            const m = byName.score < 0 && spot.olds ? Fuzzy.match(spot.olds, words, false) : byName;
            if (m.score >= 0)
                found.push({ item: spot.prepared, score: m.score, grade: m.grade, row: spot });
        }
        return Fuzzy.keep(found).map(f => Object.assign({ grade: f.grade, tier: 2 }, f.row));
    }

    // Goes to a spot: from the root into each step of its trail, found again
    // by what it is, then onto the spot itself. A step gone since, such as a
    // headset turned off, stops it where it got to.
    function goTo(spot) {
        held = [];
        restart("");
        for (const step of spot.trail) {
            // From the top, a place or an answer home is, such as Disk;
            // inside a collection, its things and then its parts, as it
            // shows them, so Agents leads on to Orchestrators.
            const from = held.length > 0 ? held[held.length - 1].row.from : null;
            const rows = held.length === 0 ? categories.concat(home())
                : from ? offered.concat(partsOf(from)) : offered;
            const row = rows.find(one => steadyId(one) === step) ?? null;
            if (row === null || !enter(row))
                return;
        }
        const at = matches.findIndex(row => steadyId(row) === spot.spot);
        if (at >= 0) {
            current = at;
            list.positionViewAtIndex(at, ListView.Contain);
        }
    }

    // The catalogue (launcher/Catalogue.js): every place and every thing's
    // actions, walked from home by the rows the launcher shows, so what asks
    // from outside does what Return would. Each id is the row's path, its
    // steps' steady ids joined by a "\u203a", which no key holds.
    readonly property string catalogueStep: "\u203a"

    function catalogue() {
        const out = [];
        const seen = new Set();
        const walk = (rows, above, ids, names, depth) => {
            for (const row of rows.filter(one => Catalogue.listed(one))) {
                const path = ids.concat([steadyId(row)]);
                const id = path.join(catalogueStep);
                if (!seen.has(id)) {
                    seen.add(id);
                    out.push(Catalogue.entry(row, id, names));
                    if (Catalogue.descends(row, depth, above))
                        walk(catalogueHolds(row, above === null || collects({ row: above })),
                            row, path, names.concat([row.name]), depth + 1);
                }
            }
        };
        walk(home(), null, [], [], 0);
        return out;
    }

    // What a row holds as `enter` shows it stepped into: from home or a
    // collection (`things`), what can be done to it, then its parts, a
    // collection's own among them, as a step into it lists them (Agents'
    // Orchestrators, Quests, Runs and AI spend); from a thing's own verbs,
    // what each is done to. One whose rows can't be made out of the
    // launcher's step holds nothing here.
    function catalogueHolds(row, things) {
        try {
            const making = things ? row.verbs : row.targets;
            const own = making ? making() ?? [] : [];
            const places = things && row.from ? partsOf(row.from) : [];
            return things ? own.concat(places, partsIn(row)) : own;
        } catch (error) {
            return [];
        }
    }

    // The rows an id is the path to, from home, or null for one not there.
    function catalogued(id) {
        let rows = home();
        let things = true;
        const path = [];
        for (const step of String(id).split(catalogueStep)) {
            const row = rows.find(one => Catalogue.listed(one) && steadyId(one) === step) ?? null;
            if (row === null)
                return null;
            path.push(row);
            rows = catalogueHolds(row, things);
            things = collects({ row: row });
        }
        return path.length > 0 ? path : null;
    }

    // Does what an id names, as asked from outside, saying what it did: a
    // place opens, an action is done, and one that ends something is shown
    // ready, selected, for Joel's own Return. Said in the journal too, as
    // the launcher's picks are.
    function catalogueRun(id, words) {
        const path = catalogued(id);
        if (path === null)
            return { error: `the launcher has nothing called ${id}` };
        const target = path[path.length - 1];
        const said = path.map(row => row.name).join(" \u203a ");
        const ids = path.map(row => steadyId(row));
        console.info(`launcher: asked from outside to ${Catalogue.kindOf(target)} ${said}`);
        if (Catalogue.kindOf(target) === "place") {
            open = true;
            goTo({ trail: ids, spot: "" });
            return { done: true, said: `opened ${said}` };
        }
        if (Catalogue.disruptive(target)) {
            open = true;
            goTo({ trail: ids.slice(0, -1), spot: ids[ids.length - 1] });
            return { done: false, offered: true, said: `${said}, ready for Return` };
        }
        if (target.given) {
            if ((words ?? "") === "")
                return { error: `${said} takes words: ${target.ask ?? ""}` };
            target.given(words);
            return { done: true, said: said };
        }
        // As Return does it, save that what's typed, Ask's question, picks
        // nothing: one whose outcome takes a while keeps the launcher, and
        // the rest close it first.
        trials.picked(target, "", path.length - 1);
        if (target.stays) {
            target.run();
            return { done: true, said: said };
        }
        if (target.now)
            target.run();
        open = false;
        if (target.now)
            return { done: true, said: said };
        if (target.later) {
            pending = target;
            later.restart();
        } else
            target.run();
        return { done: true, said: said };
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
    // A place by its name, its one word, so its name and a space step into
    // it: "machine " is Machine.
    function categoryFor(word) {
        const said = word.toLowerCase();
        return Words.categories.find(category => category.name.toLowerCase() === said) ?? null;
    }

    // What a place holds, in the order the vocabulary gives, which is by
    // importance and relation (attachments/collection-order.md under the
    // launcher's quest; Joel, 2026-10-25: "ordered intelligently by
    // importance and relation"): each thing as it's found anywhere, only
    // those this machine has, then the typed commands whose answers are
    // this place's, last, so they remind him what can be typed here.
    function members(category) {
        return (category.holds ?? []).map(id => resolved(id)).filter(row => row !== null)
            .concat(commandsIn(category));
    }

    // The typed commands a place lists, in its order.
    function commandsIn(category) {
        const named = category.commands ?? [];
        return commands().filter(row => named.includes(row.key))
            .sort((a, b) => named.indexOf(a.key) - named.indexOf(b.key));
    }

    // What a category holds, said after its name at home so scrolling
    // through them says what's there: its members' names by name, as
    // their rows name them ("Agents, AI spend, Calls, Capture"), as many
    // whole as fit in `room` characters, but for one its own name already
    // says (Sound's Sound). Read from the vocabulary as home is made, so no
    // answer is asked for it.
    function holding(category, room) {
        const names = (category.holds ?? []).map(id => heldName(id)).filter(name => name !== "")
            .concat(commandsIn(category).map(row => row.start.trim()
                || row.key.replace(/^command:/, "")));
        const own = (category.name ?? category.id).toLowerCase();
        return listed(names.filter(name => name.toLowerCase() !== own), room);
    }

    // Names as one line, as many whole as fit in `room` characters, then
    // an ellipsis for the rest.
    function listed(cased, room) {
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
        const place = placeNamed(id);
        if (place)
            return place.name;
        const known = keyed[id];
        if (known && !known.category)
            return known.name;
        const entry = Words.of(id);
        const there = sources.some(one => one.settings.name === (entry.provider ?? id));
        return there && !entry.name.includes("{") ? entry.name : "";
    }

    // A category another holds, as Machine holds Power, by its id: its own
    // row, stepped into as one at home is. Undefined for anything else.
    function placeNamed(id) {
        return categories.find(row => row.category.id === id);
    }

    // A thing the vocabulary names, as a row: a category, the row of that
    // key, a collection whole, an answer to its question, or a provider's
    // things as one row holding them. Null where this machine has none.
    function resolved(id) {
        const place = placeNamed(id);
        if (place)
            return Object.assign({}, place, {
                count: holding(place.category, Math.max(1, rowRoom() - place.name.length))
            });
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
            return parted(said) ? inPlace(Object.assign(answered(heard(said, source, question)), {
                is: `answer:${id}`,
                chip: question
            }), entry.name.includes("{") ? "" : entry.name) : null;
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
            return row;
        });
    }

    // A collection's parts the vocabulary names: Agents holds its tasks,
    // helpers and spend, the Keyboard its keyboards and shortcuts.
    function partsOf(source) {
        return Words.of(source.settings.name).parts.map(id => resolved(id))
            .filter(row => row !== null);
    }

    // Rows found by what's typed, inside a step, a category or a
    // collection's parts, in the top level's tiers (Rank.js): a name that
    // is what's typed, then one starting with it, then one with a later
    // word starting with it, and then a row found only by what it holds of
    // its own (hiddenWords), each tier best match first. Such a word isn't
    // said beside the row, since it may be a name the machine knows a thing
    // by, such as tailscale0, that Joel never sees.
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

    // A row's other words, prepared to match: its keywords, which say only
    // what it holds of its own, such as a network card's name, since the
    // vocabulary gives a thing one word, its name. Made once for each row
    // and kept on it, since a step's rows are searched at every key; a
    // WeakMap keeping them crashed the shell once its rows were collected.
    function hiddenWords(row) {
        if (row.hiddenMade !== undefined)
            return row.hiddenMade;
        const words = row.keywords ? row.keywords.text ?? "" : "";
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
    // computer, "battery".
    function answeringIn(category, letters) {
        if (letters.trim() === "")
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
    // not a search inside it, so the field gives it back whole. An answer
    // there only when nothing else is, as Ask's for any two plain words,
    // asks nothing whole, or "network a" would leave Network on its
    // second letter.
    function askedWhole(letters) {
        if (held.length !== 1 || Fuzzy.words(letters).length === 0)
            return "";
        const whole = `${held[0].typed} ${letters}`;
        return sources.some(source => {
            const said = source.answer(whole);
            return !!said && !said.fallback;
        }) ? whole : "";
    }

    // The places a category sits in, outermost first, as Machine holds
    // Power; none for one at home.
    function placesAbove(category) {
        const above = Words.categories.find(one => (one.holds ?? []).includes(category.id));
        return above ? placesAbove(above).concat([above]) : [];
    }

    // Steps into a category from a word and a space, or from the top,
    // through the places it sits in, so the path says where it lives and
    // the left arrow climbs out through them; what was typed is put back on
    // the way out.
    function intoCategory(category, word) {
        const chain = placesAbove(category).concat([category]);
        const steps = [];
        // Where each step was stepped in from: the field for the first,
        // then the place before it, its row for the next one selected.
        let from = Object.assign(here(), { typed: word });
        chain.forEach((place, at) => {
            const rows = members(place);
            steps.push(Object.assign(from, {
                row: categories.find(one => one.category === place),
                rows: rows
            }));
            const inner = rows.findIndex(one => one.category === chain[at + 1]);
            from = { typed: "", at: Math.max(0, inner), key: inner < 0 ? ""
                : steadyId(rows[inner]), scroll: 0 };
        });
        held = steps;
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
        row.keywords = Fuzzy.prepare([row.name, collection.noun].join(" "));
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
        // What each collection took, in milliseconds, for a key that's slow
        // to say which (Trials.qml).
        const took = {};
        listsTook = took;
        for (const source of among ?? collections) {
            // One opened only by its own chord adds no row to a search, but
            // does inside a category that holds it.
            if (source.collection.searched === false && !among)
                continue;
            const asked = Date.now();
            const said = source.gather(letters);
            took[source.settings.name] = Date.now() - asked;
            // Inside a category, its best is one named what's typed before
            // one picked for it.
            const learned = learnedFirst(said.rows, letters);
            const found = among ? namedFirst(learned, letters) : learned;
            if (found.length === 0)
                continue;
            const best = found[0];
            const noun = source.collection.noun;
            const gathered = Object.assign({}, best, {
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
            });
            // One with no act of its own, only more inside (an
            // orchestrator, a device), lands inside it on Return, its
            // collection's crumb before its own, as the approved proposal's
            // one feel has it; the right arrow still lists all it found.
            if (!best.run && (best.verbs || best.targets))
                Object.assign(gathered, {
                    stays: true,
                    run: () => root.landInside(gathered, best)
                });
            rows.push(gathered);
        }
        return rows;
    }

    // Steps into a collection's matches, and then into the one of them that
    // was found, by what it is.
    function landInside(gathered, best) {
        if (!enter(gathered))
            return;
        const at = matches.findIndex(row => steadyId(row) === steadyId(best));
        if (at < 0)
            return;
        current = at;
        enter(matches[at]);
    }

    // How tall the list is while the launcher's open: always its rows, and
    // home's gap when it has one.
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

    // What the keys do for a row, as the foot says them, a key and its
    // word each: Return's verb, Shift+Return where it can, where the arrows
    // step, and Tab at home. The left arrow is said only where it goes
    // back: that it takes away what's typed is what any field does, and
    // saying so took room from how things stand as soon as Joel typed.
    function keysFor(choice) {
        // A row whose thing has gone does nothing, so it says no key.
        if (!choice || choice.gone)
            return [];
        const keys = [["\u21b5", verbOf(choice)]];
        if (held.length === 0 && choice.below)
            keys.push(["\u21e7\u21b5", "below"]);
        if ((held.length === 0 || collecting) && choice.verbs)
            keys.push(["\u2192", choice.from ? "see all" : "more"]);
        if (typed === "" && held.length > 0)
            keys.push(["\u2190", "back"]);
        return keys;
    }

    // How far in the foot's keys may reach and how wide Return's place is,
    // so they hold still. Said for the selected row alone, the keys moved
    // as the arrows went from a row that opens to one with more behind it,
    // and squeezed how things stand beside them from words to glyphs and
    // back (Joel, 2026-10-25: "jumpy ... at home arrowing around"). So
    // Return, last at the right end, keeps a place as wide as the longest
    // word it's said since the launcher opened, and how things stand keeps
    // clear of where the widest set of keys has reached. The rest sit just
    // before Return, each as wide as it is: a place kept for every key any
    // row had left holes between them as soon as Joel typed ("the
    // whitespacing ... is all fucked up"), and took room from how things
    // stand that no row used. Both only grow while the
    // launcher's open, and only as the rows change, so nothing gives way
    // under the selection. They open as wide as most rows need, Return's
    // place as wide as "connect" and the keys as wide as a program's at
    // the top, so the first letter typed doesn't move them: grown only as
    // rows came, Return moved in as soon as Joel typed, since home's rows
    // say less than what a letter finds.
    readonly property string footVerbRoom: "connect"
    readonly property var footKeysRoom: [["\u21b5", footVerbRoom], ["\u21e7\u21b5", "below"],
        ["\u2192", "see all"]]
    property string footVerbWidest: footVerbRoom
    property real footKeysWidest: 0

    function footReset() {
        footVerbWidest = footVerbRoom;
        footKeysWidest = keysWide(footKeysRoom);
    }

    // How wide a row's keys are, Return in its place.
    function keysWide(told) {
        if (told.length === 0)
            return 0;
        return told.slice(1).reduce((sum, key) => sum + keys.spacing
            + measure.advanceWidth(`${key[0]} ${key[1]}`), returnPlace(told[0]));
    }

    // How wide Return's place is beside what it says now.
    function returnPlace(key) {
        return Math.max(measure.advanceWidth(`${key[0]} ${key[1]}`),
            measure.advanceWidth(`${key[0]} ${footVerbWidest}`));
    }

    function noteKeys() {
        const rows = asking ? matches.concat([asking]) : matches;
        const said = rows.map(row => keysFor(row)).filter(keys => keys.length > 0);
        const verb = said.reduce((longest, keys) => keys[0][1].length > longest.length
            ? keys[0][1] : longest, footVerbWidest);
        if (verb !== footVerbWidest)
            footVerbWidest = verb;
        const wide = Math.max(footKeysWidest, ...said.map(keys => keysWide(keys)));
        if (wide > footKeysWidest)
            footKeysWidest = wide;
    }

    // The widest that graphed rows have said at their right in this step,
    // in characters, so the graphs' column only ever widens until a key
    // changes the step: worked out afresh from the rows shown, it moved
    // every graph and cut every name somewhere else each time a figure
    // grew or shrank by a digit.
    property int figuresWidest: 0
    property string figuresStep: ""

    function noteFigures() {
        const step = `${held.map(each => steadyId(each.row)).join("/")}|${typed}`;
        const now = Math.max(0, ...matches.filter(row => !!row.graph).map(row => ((held.length === 0
            ? kindOf(row) : "") || ((row.chord ?? []).length > 0 ? "" : row.side ?? "")).length));
        if (step !== figuresStep) {
            figuresStep = step;
            figuresWidest = now;
        } else if (now > figuresWidest)
            figuresWidest = now;
    }

    // The shortest way the foot's facts have had to read since the launcher
    // opened (Standing.js's fittedFrom), which they never read longer than
    // until it closes.
    property int footStage: 0

    // A step of its own has notes of its own, so the field's right end
    // takes the room they need afresh (Prompt.qml's steady). Known by the
    // path, since a refresh makes the same steps again.
    readonly property string stepsTaken: held.map(step => steadyId(step.row)).join("/")
    onStepsTakenChanged: prompt.forget()

    onMatchesChanged: {
        noteKeys();
        noteFigures();
    }
    onAskingChanged: noteKeys()

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

    // The names a row had before the glossary renamed it (words.json's
    // `once`, or the row's own for one whose key isn't a word's, as a
    // connection's): they still find it, and are never shown, so a rename
    // never loses a row (Joel, 2026-10-26: "i didn't know its name had
    // changed, i was used to calling it caffeinate").
    function oldNames(row) {
        const id = String(row.key ?? "").replace(/^(?:category|collection|answer|place):/, "");
        return row.once ?? row.category?.once ?? Words.of(id).once ?? [];
    }

    // "Once called Caffeinate." below a row found by a name it had and not
    // by its own, so the new name is learned as it's used.
    function onceCalled(row) {
        const typed = Fuzzy.plainly(root.typed);
        if (typed === "" || startsAWord(row.name ?? "", typed))
            return "";
        const old = oldNames(row).find(name => {
            const plain = Fuzzy.plainly(name);
            return plain.startsWith(typed) || typed.startsWith(plain);
        });
        return old ? `Once called ${old}.` : "";
    }

    // A row's own words, with the names it had and those a machine adds
    // for it (jitsusama.quickshell.launcher.synonyms), so a habit of Joel's
    // finds it. Made once for each row, and never twice for the same one.
    function withSynonyms(row) {
        const more = (Providers.launcher.synonyms[row.key] ?? []).concat(oldNames(row));
        if (more.length === 0 || row.synonymised)
            return row;
        row.keywords = Fuzzy.prepare([row.keywords ? row.keywords.text : ""].concat(more)
            .join(" ").trim());
        row.synonymised = true;
        return row;
    }

    // How often each thing's been picked, taken as the launcher opens, so
    // what it orders holds still while it's open.
    property var often: ({})

    // With nothing typed, home is the same rows every time, each in its
    // one place, as the approved proposal sets them
    // (attachments/launcher-proposal.md under the launcher's quest): Now
    // first, saying what's on or waiting this moment, then each place, a
    // context of its own, saying what it holds, so one glance reminds Joel
    // what there is. Each thing has one home among them, and nothing else
    // is here. What each row says is read as the launcher opens and held
    // while it's open, so nothing moves under him.
    // How many characters a row's name and what it holds have room for.
    function rowRoom() {
        return Math.floor((list.width - Theme.space.l - Theme.space.xxl
            - Theme.space.xxl - Theme.space.m - Theme.space.l)
            / Math.max(1, measure.advanceWidth("0")));
    }

    function home() {
        const room = rowRoom();
        const rows = Words.home.map(id => homeRow(id, room)).filter(row => row !== null);
        // Now stands apart from the rest by a gap, as what's this moment.
        if (rows.length > 1)
            rows[1] = Object.assign({}, rows[1], { gap: true });
        return rows;
    }

    // One of home's rows: a category, or a collection whole, Now and
    // Programs, each saying what it holds by name.
    function homeRow(id, room) {
        const category = categories.find(row => row.category.id === id);
        if (category)
            return Object.assign({}, category, {
                count: holding(category.category, room > 0 ? room - category.name.length
                    : Infinity)
            });
        const source = collections.find(one => one.settings.name === id);
        if (!source)
            return answerPlace(id);
        const row = whole(source, false, false);
        // In Now, what waits on Joel leads, named by the first of it, since
        // it's the one thing there only he can move; the rest follow by name.
        const lead = source.foremost ?? "";
        const names = source.everything().filter(one => lead === "" || one.key !== "on:waiting")
            .map(one => one.name);
        const left = room > 0 ? room - row.name.length : Infinity;
        const rest = names.length === 0 ? ""
            : listed(names, lead === "" ? left : left - lead.length - " \u00b7 ".length);
        return Object.assign({}, row, {
            count: lead === "" && rest === "" ? source.collection.empty ?? ""
                : [lead, rest].filter(part => part !== "").join(" \u00b7 ")
        });
    }

    // A place that's one answer, as Disk is: named as the vocabulary names
    // it, what it says now after its name, and its parts a step in. Until
    // the machine can say, as before its first reading is back, it's there
    // by name with nothing after it, so home never gains a row while it's
    // open, and Return leaves it open rather than closing on nothing.
    function answerPlace(id) {
        const entry = Words.of(id);
        if (!entry.name)
            return null;
        const row = resolved(id);
        if (row)
            return Object.assign({}, row, { name: entry.name, count: row.side ?? "", side: "" });
        return {
            key: `place:${id}`,
            name: entry.name,
            glyph: Words.glyph(id),
            hue: "voice",
            plain: true,
            chord: [],
            side: "",
            count: "",
            verb: "open",
            stays: true,
            run: () => {}
        };
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
            note: source.noted(),
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
        if (!row || row.gone)
            return false;
        if (held.length > 0 && row.given) {
            ask(row);
            return true;
        }
        // A place found from the top that sits in another is stepped into
        // through it, so the path says where it lives.
        if (held.length === 0 && row.category && placesAbove(row.category).length > 0) {
            intoCategory(row.category, typed);
            return true;
        }
        const things = held.length === 0 || collecting;
        const own = things ? (row.verbs ? row.verbs() : null) : row.targets ? row.targets() : null;
        // A thing's parts come after what can be done to it.
        const parts = things ? partsIn(row) : [];
        const into = parts.length > 0 ? (own ?? []).concat(parts) : own;
        // A collection with nothing in it still holds the places its
        // vocabulary names, which a step into it shows: Agents with no
        // agent running still leads to its Quests and Orchestrators.
        const places = things && row.from ? partsOf(row.from).length : 0;
        if ((!into || into.length === 0) && places === 0)
            return false;
        held = held.concat([Object.assign(here(), { row: row, rows: into ?? [] })]);
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
        // A provider that throws mid-search mustn't leave every later search
        // keeping its rows as a refresh does.
        steadying = true;
        try {
            search(typed);
        } finally {
            steadying = false;
        }
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

    // What each opening took, to tell the homes Joel tries apart.
    Trials {
        id: trials
    }

    // Every choice the words find (Fuzzy.js says how), in Rank.js's order:
    // what's latched for what's typed, then by tier, then what was picked
    // for it, what's picked most and the better match. One found only with
    // its keywords, beside its name or alone, is in the last tier, but for a
    // collection's own words, which are what it's called.
    // Rows gone since they were shown leave every step once a key searches
    // afresh, so a refresh after it doesn't keep them on as gone again.
    function forgetGone() {
        if (held.some(step => step.rows.some(row => row.gone)))
            held = held.map(step => Object.assign({}, step, {
                rows: step.rows.filter(row => !row.gone)
            }));
    }

    function search(letters) {
        if (!steadying)
            forgetGone();
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
        // A category's word and a space step into it, unless a name goes on
        // from that word, as Power off does from Power: then it's searched
        // for here, and nothing moves under what's being typed.
        const chipped = held.length === 0 ? /^(\S+) $/.exec(letters) : null;
        const named = chipped && !choices.some(row => !row.category
            && String(row.name ?? "").toLowerCase().startsWith(letters))
            ? categoryFor(chipped[1]) : null;
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
            // tasks, helpers and spend. A collection whose vocabulary says
            // its parts lead ("lead": "parts") lists them first, so the
            // Keyboard's keyboards aren't below a hundred keys; it's said,
            // not counted, so no list changes order as it grows. A part
            // named leads what's found.
            let found = [];
            if (from && words.length === 0) {
                const members = from.everything();
                const parts = partsOf(from);
                found = Words.of(from.settings.name).lead === "parts" ? parts.concat(members)
                    : members.concat(parts);
            } else if (from)
                found = within(partsOf(from), letters)
                        .concat(learnedFirst(from.search(letters).rows, letters));
            // A step's rows, all of them in their order until something's
            // typed, then those it finds, best first, and with them the
            // choices of a part that offers them (a display's Scale), so
            // what's typed reaches them without a step in: "scale 150".
            // Found by name or by what it holds, in the top level's tiers.
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
        // What the search spends its time on, said with a key that's slow
        // (Trials.qml): every collection searched, every row matched, then
        // the answers and the list made of them.
        const began = Date.now();
        gathered = gathering(letters);
        const listed = Date.now();
        latch = "";
        foundHere = {};
        // What the rows' time went on, for a first letter that's slow
        // (Joel's own openings put 215 to 271 ms there): matching every
        // choice, ordering what was found, or making the rows shown.
        let costs = null;
        if (words.length === 0) {
            candidates = [];
            root.found = home();
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            // What the next letter narrows from: everything found, and a
            // collection held back only for being too short yet.
            const early = [];
            const looked = narrowing ? candidates : choices;
            for (const choice of looked) {
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
            const matchedAll = Date.now();
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
            const orderedAll = Date.now();
            const made = ordered.map(f => Object.assign({
                byWord: f.byWord === true,
                times: (learned[f.choice.key] ?? { times: 0 }).times,
                grade: f.grade,
                tier: f.tier,
                kept: !!learned[f.choice.key]
            }, f.choice));
            const madeAll = Date.now();
            root.found = made.concat(spotted(letters));
            costs = { matching: matchedAll - listed, ordering: orderedAll - matchedAll,
                making: madeAll - orderedAll, spots: Date.now() - madeAll, looked: looked.length,
                found: found.length };
        }
        const matched = Date.now();
        compose(true);
        list.positionViewAtBeginning();
        searchParts = { lists: listed - began, rows: matched - listed,
            answers: Date.now() - matched, each: listsTook, within: costs };
    }

    // What the last search at the top level spent its time on, or null for
    // one inside a step.
    property var searchParts: null
    // What each collection took in the last search of them.
    property var listsTook: ({})

    // Times each key from its search to the first frame shown after it,
    // the one its new rows are in: the window says when a frame's been
    // swapped onto the screen, and it's listened to only while a key waits
    // for one.
    property bool keyWaits: false
    property real keyBegan: 0
    property real keySearched: 0
    property string keyText: ""

    function keyTimed(typed, began) {
        keyBegan = began;
        keySearched = Date.now() - began;
        keyText = typed;
        keyWaits = true;
    }

    function keyDrawn() {
        keyWaits = false;
        trials.timed(keySearched, Date.now() - keyBegan, searchParts, keyText);
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
        // Its thing has gone, so there's nothing left to do it to.
        if (choice.gone)
            return;
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
            trials.picked(choice, typed, held.length);
            into(choice.opens);
            return;
        }
        // The clipboard takes text only from a program holding the keys.
        if (choice.now)
            choice.run();
        trials.picked(choice, typed, held.length);
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

        // The catalogue, every place and action by id, as one line of JSON.
        function list(): string {
            return JSON.stringify(root.catalogue());
        }

        // Does what an id names, or shows it ready when it ends something,
        // saying what it did as JSON.
        function run(id: string): string {
            return JSON.stringify(root.catalogueRun(id, ""));
        }

        // The same, for an action that takes words, such as Talk.
        function runWith(id: string, words: string): string {
            return JSON.stringify(root.catalogueRun(id, words));
        }

        // Where every word on the card is drawn, for the check that nothing
        // moves without changing.
        function laidOut(): string {
            return root.laidOut();
        }
    }

    // Every word drawn in an item, the item itself and all inside it, each
    // with where it's drawn on the card: its box's left and top and where
    // its letters start, which an aligned text moves within its box.
    function wordsIn(item, into) {
        if (!item || !item.visible || item.opacity === 0)
            return [];
        const words = [];
        if (typeof item.text === "string" && typeof item.contentWidth === "number"
            && item.text !== "" && item.width > 0) {
            const at = item.mapToItem(into, 0, 0);
            const spare = Math.max(0, item.width - item.contentWidth);
            const inset = item.horizontalAlignment === Qt.AlignRight ? spare
                : item.horizontalAlignment === Qt.AlignHCenter ? spare / 2 : 0;
            words.push({ text: item.text, x: Math.round(at.x), y: Math.round(at.y),
                starts: Math.round(at.x + inset) });
        }
        for (const child of item.children)
            for (const word of wordsIn(child, into))
                words.push(word);
        return words;
    }

    // Where the card's words are, as JSON: the field's, the foot's, and each
    // row's shown, with what the row is and how it reads, so a test can say
    // a row that reads the same was drawn in the same place (Joel,
    // 2026-10-25: "text alignment and shit constantly shifting around").
    function laidOut() {
        const rows = [];
        for (let at = 0; at < root.matches.length; at += 1) {
            const drawn = list.itemAtIndex(at);
            const row = root.matches[at];
            if (drawn && drawn.visible)
                rows.push({ at, is: Rank.identity(row), selected: at === root.current,
                    reads: [row.name, row.count ?? "", row.side ?? "", root.kindOf(row)]
                        .join(" \u00b7 "), words: wordsIn(drawn, card) });
        }
        return JSON.stringify({ typed: root.typed, field: wordsIn(prompt, card),
            foot: wordsIn(foot, card), rows });
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
                gone: !!row.gone,
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
            glance: standing.said,
            // Where Return's key starts, which holds still as the arrows go.
            keysAt: Math.round(foot.returnAt),
            // How far the list is scrolled, which only a key changes.
            scrolled: Math.round(list.contentY)
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
            trials.opened();
            pointer = Qt.point(-1, -1);
            footReset();
            footStage = 0;
            figuresStep = "";
            held = [];
            prompt.forget();
            often = picks.counts();
            // What's inside things is read again for each opening, the
            // first time a search needs it.
            spotsMade = false;
            spots = [];
            restart("");
            // Always the same height, eight rows and home's gap, kept until
            // it closes, so the card is the same shape however much home
            // holds.
            tall = rows * Theme.space.xxxl + (matches.slice(0, rows).some(row => row.gap)
                ? Theme.space.m : 0);
            // Before niri hands over the keys, so the first one lands.
            prompt.take();
            Report.say(`stands ${standing.said}`);
        } else {
            screenHere = focusedScreen;
            trials.closed();
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

        Connections {
            target: card.Window.window
            enabled: root.keyWaits

            function onFrameSwapped() {
                root.keyDrawn();
            }
        }

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
                        steady: true
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
                        onTextChanged: {
                            trials.retyped(root.typed, text);
                            const began = Date.now();
                            root.searchParts = null;
                            root.search(text);
                            if (!root.asking)
                                root.keyTimed(text, began);
                        }
                        onAccepted: below => {
                            trials.pressed("returns");
                            if (root.asking)
                                root.give();
                            else
                                root.run(root.selected, below);
                        }
                        // Tab on an answer puts the answer in place of the
                        // question, so a sum can be carried on; one still
                        // being asked for leaves the question as it is.
                        onCompleted: {
                            trials.pressed("tabs");
                            // At home there's nothing to complete: home
                            // is the categories, a row each, and an arrow
                            // reaches any of them (Joel, 2026-10-25: "i
                            // don't like tab for browse, that seems
                            // useless").
                            if (root.atHome)
                                return;
                            // A command puts its start in the field.
                            if (root.current !== -1 && root.selected?.start !== undefined)
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
                        onForward: {
                            trials.pressed("steps");
                            root.enter(root.matches[root.current]);
                        }
                        onBackward: {
                            trials.pressed("backs");
                            root.back();
                        }
                        // The highest the selection goes: an answer's headline
                        // sits above the rows, at -1.
                        readonly property int highest: root.answer && !root.parted(root.answer)
                            ? -1 : 0
                        Keys.onUpPressed: {
                            trials.pressed("arrows");
                            root.current = Math.max(root.current - 1, highest);
                        }
                        Keys.onDownPressed: {
                            trials.pressed("arrows");
                            root.current = Math.max(highest,
                                Math.min(root.current + 1, root.matches.length - 1));
                        }
                        Keys.onPressed: event => {
                            if (event.key === Qt.Key_PageUp) {
                                trials.pressed("arrows");
                                root.current = Math.max(root.current - root.rows, highest);
                            } else if (event.key === Qt.Key_PageDown) {
                                trials.pressed("arrows");
                                root.current
                                    = Math.min(root.current + root.rows, root.matches.length - 1);
                            }
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
                            // have said at their right in this step, as Choice
                            // says it, so their graphs stand in one column that
                            // holds still as 9% becomes 10% (root.figuresWidest).
                            readonly property int figures: root.figuresWidest
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
                                    // Gone since it was shown, it stays dimmed
                                    // in its place until a key (Steady.js).
                                    quiet: !!entry.modelData.gone
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
                        readonly property var told: root.keysFor(choice)
                        // As the foot says them: Return last, at the right
                        // end, the rest before it. Return first, in a place
                        // kept for the widest set, left holes after the keys
                        // a row didn't have; at the end, what Return says
                        // keeps to the right edge and the others come and
                        // go only to its left, as an action bar's main act
                        // does.
                        readonly property var shown: told.length === 0 ? []
                            : told.slice(1).concat([told[0]])
                        readonly property real returnPlace: told.length === 0 ? 0
                            : root.returnPlace(told[0])
                        // Where Return's place starts, which holds still.
                        readonly property real returnAt: width - Theme.space.l - returnPlace
                        // How far in the keys may reach: as far as the
                        // widest of them has needed since the launcher
                        // opened, so the left's room holds still too.
                        readonly property real keysFrom: width - Theme.space.l
                            - Math.max(root.footKeysWidest, keys.implicitWidth)
                        // The room the left has, short of the keys.
                        readonly property real room: keysFrom - Theme.space.xl - Theme.space.l

                        // An answer's footer says where it came from.
                        readonly property string said: {
                            if (!choice)
                                return "";
                            if (choice.state)
                                return choice.state();
                            const once = root.onceCalled(choice);
                            if (once !== "")
                                return once;
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

                            // Read no longer than the shortest they've had to
                            // since the launcher opened (root.footStage), so
                            // they change their form once, not back and forth.
                            readonly property var fit: {
                                const wide = facts => facts.reduce((sum, fact) => sum + measure.advanceWidth(fact.said), 0)
                                    + Math.max(0, facts.length - 1) * spacing;
                                return Standing.fittedFrom(facts, told => wide(told) <= foot.room,
                                    root.footStage);
                            }
                            readonly property var kept: fit.told

                            // Noted once the binding has settled, since `fit`
                            // reads what this writes.
                            onFitChanged: Qt.callLater(standing.noteStage)

                            function noteStage() {
                                if (root.open && foot.room > 0 && fit.stage > root.footStage)
                                    root.footStage = fit.stage;
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
                            x: foot.width - Theme.space.l - implicitWidth
                            anchors.verticalCenter: parent.verticalCenter
                            // The selected row's keys, Return last in its
                            // place (footVerbWidest), the rest close by.
                            keys: foot.shown
                            endsRight: true
                            widths: foot.shown.map((key, at) => at === foot.shown.length - 1
                                ? foot.returnPlace : 0)
                        }
                    }
                }
            }
        }
    }
}
