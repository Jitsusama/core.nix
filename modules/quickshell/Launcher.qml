// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return runs it, or Shift+Return opens it below the
// focused window. Tab takes the grey completion the command line shows, and
// does nothing while it shows none, as at home, where the arrows already
// reach every category. Each
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
            source.thingsChanged.connect(root.stale);
            made.push(source);
        }
        sources = made;
        freshen();
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
        const listed = root.found.concat(stillActed(root.found));
        // An answer only there when nothing else is, as asking a model by a
        // question's first word is, gives way to anything found.
        answer = said && said.fallback
            && (listed.some(row => !row.category) || gathered.length > 0) ? null : said;
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
        const things = new Set(listed.filter(row => !row.category)
            .map(row => Fuzzy.plainly(row.name)));
        const found = listed.filter(row => !row.category || atHome
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
        matches = steadying ? steadied(rows.concat(below)) : rows.concat(below);
        // An answer only started gives way to a name that starts the same,
        // its first word: "pow" is still Power Off. A later word starting
        // so is no name Joel was typing, and gave "batt" to a copy that
        // said "50%, on battery", above the battery's own answer.
        const meant = answer && !(answer.question && below.length > 0
            && String(below[0].name).toLowerCase().startsWith(Fuzzy.plainly(typed)));
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
        const at = current;
        const scroll = list.contentY;
        compose(false);
        if (list.contentY !== scroll)
            list.contentY = scroll;
        root.showMoved(at);
    }

    // After a refresh, the list stays where it was and moves only to show a
    // selection the refresh itself moved, through reveal, so it glides and
    // never restarts a glide headed there. Brought to the selection on
    // every refresh instead, the list snapped mid-glide to where the glide
    // was going and was pulled back for the rest of it, each second inside
    // Machine, and a list scrolled by hand went back under the pointer
    // (Joel, 2026-10-29: "a lot of scrolling focus quirkiness").
    function showMoved(was) {
        if (current >= 0 && current !== was)
            list.reveal(current);
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
                rows: Steady.kept(was.rows, led(row, row.verbs()), steadyId)
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
    // Every source's things, then each collection and category, as one
    // list: what the search finds from, and what keyed, partsOfRows and
    // namedAs are made of. It's joined by hand rather than bound, since a
    // binding subscribes to everything it reads however deep, and every
    // source's things and each collection's note read live state (the
    // machine queue, Niri's focus, the sessions): bound, it was made
    // again whole on each of those with the launcher closed, two thirds
    // of the shell's idle processor (2026-10-29). A change marks it
    // stale; it's joined again as the launcher opens, before anything
    // asks the catalogue from outside, and while open once the changes
    // settle, in one pass, so home, stepping in and a refresh in place
    // always read it whole.
    property var choices: []
    property bool choicesStale: true
    // How many times it's been joined, for the check that a closed
    // launcher joins nothing.
    property int choicesJoined: 0
    function stale() {
        choicesStale = true;
        if (open)
            Qt.callLater(freshen);
    }
    function freshen() {
        if (!choicesStale)
            return;
        choicesStale = false;
        const out = [];
        for (const source of sources) {
            for (const row of source.things) {
                if (!(row.key.startsWith("action:") && standIns.includes(row.name)))
                    out.push(withSynonyms(row));
            }
        }
        for (const source of collections)
            out.push(withSynonyms(named(source)));
        for (const row of categories)
            out.push(withSynonyms(row));
        choicesJoined += 1;
        choices = out;
    }
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
    // times over. Made a moment after each closing (spotsLater), and by
    // the first search of three letters or more when none were made yet,
    // of the collections whose things are devices, few and read whole, so
    // a search never reads them again.
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
    // headset turned off, stops it where it got to. Each step's row is
    // selected before it's stepped into, and the first step keeps what was
    // typed and selected before it, so the left arrow climbs back out
    // through every step and onto the search it came from, as any way in
    // does (Joel, 2026-10-26: every way in climbs back out the same way).
    function goTo(spot) {
        const came = here();
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
            const listedAt = matches.findIndex(one => steadyId(one) === step);
            if (listedAt >= 0)
                current = listedAt;
            const first = held.length === 0;
            if (row === null || !enter(row))
                return;
            if (first)
                held = [Object.assign({}, held[0], came)].concat(held.slice(1));
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

    // Set while the catalogue is walked from outside, so a collection that
    // makes its rows only while the launcher is open (Agents) makes them
    // for the walk too.
    property bool cataloguing: false

    function catalogue() {
        cataloguing = true;
        try {
            return catalogueWalked();
        } finally {
            cataloguing = false;
        }
    }

    function catalogueWalked() {
        freshen();
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

    // Every row Return does nothing on, and how many rows there were,
    // from home down as stepping in
    // shows them, each with where it sits, for the check that fails any
    // (Joel, 2026-10-26: every row has a Return act). It walks deeper
    // than the catalogue and into each program, window and quest, but not
    // into the long lists, the clipboard's or the emoji, whose rows are
    // all made one way and are proven offscreen.
    readonly property int actlessDeepest: 6

    // Each thing the vocabulary places that is an answer, by its name, as
    // Time, Date and Weather are: its one row is the answer as
    // `resolved` places it, wherever it shows (Joel, 2026-10-28: "we need
    // to not have different versions of these sort of elements"). A part,
    // whose id is its holder's and its own, as a display's Brightness is,
    // shows as its holder makes it, so it isn't one.
    readonly property var answeredThings: {
        const out = {};
        for (const id of Object.keys(Words.things)) {
            const entry = Words.of(id);
            if ((entry.answers ?? []).length > 0 && (entry.in ?? "") !== "" && !id.includes(":"))
                out[entry.name] = id;
        }
        return out;
    }

    // Whether a row bears an answered thing's name without being that
    // thing's own row, which is a surface drawing its own version of it.
    // What an answer holds is its own, as the time's Date is the date
    // where that time is, so it isn't asked of.
    function ownVersion(row, above) {
        const id = answeredThings[row.placed ?? row.name];
        return id !== undefined && above?.answered !== true && row.is !== `answer:${id}`
            && row.key !== `place:${id}`;
    }

    // The kinds made by one component each, by how their rows are keyed,
    // and the component that makes them (docs/design.md, one component per
    // thing): a mail, a meeting, a file or a folder row made anywhere but
    // by its component is a version a surface drew of it.
    readonly property var components: [
        { keyed: /^mail:/, made: "mail" },
        { keyed: /^calendar:meeting:/, made: "meeting" },
        { keyed: /^file:/, made: "file" },
        { keyed: /^folder:/, made: "folder" }
    ]

    // A row of a kind its component makes, made otherwise, or null.
    function uncomposed(row) {
        const kind = components.find(one => one.keyed.test(row.key ?? ""));
        return kind && row.component !== kind.made ? kind.made : null;
    }

    function actless() {
        const out = [];
        const own = [];
        const loose = [];
        let composed = 0;
        // Every step whose first row is a copy where more is held, which
        // docs/design.md's Return rule never allows ("Copy is never the
        // first row of a step, unless it's the only one, and is always the
        // last"; Joel, 2026-10-28: "the copy at the top of most screens is
        // confusing").
        const copyFirst = [];
        // Every row whose Return steps in over an act of its own, where the
        // step doesn't lead with that act, so it couldn't be chosen inside
        // (Joel, 2026-10-28: "enter is supposed to be the default action,
        // but that means that action should be selectable within").
        const skipped = [];
        const seen = new Set();
        const walk = (rows, above, ids, names, depth) => {
            for (const row of rows.filter(one => !!one && (one.name ?? "") !== "")) {
                const path = ids.concat([steadyId(row)]);
                const id = path.join(catalogueStep);
                if (!seen.has(id)) {
                    seen.add(id);
                    if (!Catalogue.acts(row))
                        out.push({ id, words: row.name, in: names.join(" \u203a ") });
                    if (ownVersion(row, above))
                        own.push(`${names.concat([row.name]).join(" \u203a ")}`
                            + ` (${row.is ?? row.key})`);
                    const kind = uncomposed(row);
                    if (kind !== null)
                        loose.push(`${names.concat([row.name]).join(" \u203a ")}`
                            + ` (${row.key}, not made as a ${kind})`);
                    else if (row.component)
                        composed += 1;
                    const deeper = depth < actlessDeepest
                        && !(row.opens && Catalogue.long(row.opens));
                    const inside = deeper
                        ? catalogueHolds(row, above === null || collects({ row: above })) : [];
                    if (inside.length > 1 && copying(inside[0]))
                        copyFirst.push({ id, words: row.name, in: names.join(" \u203a ") });
                    if (inside.length > 0 && stepsOver(row)
                        && !Catalogue.saysAct(inside[0], verbOf(row).toLowerCase()))
                        skipped.push({
                            id,
                            words: `${row.name} (${verbOf(row)}, led by ${inside[0].name})`,
                            in: names.join(" \u203a ")
                        });
                    walk(inside, row, path, names.concat([row.name]), depth + 1);
                }
            }
        };
        walk(home(), null, [], [], 0);
        // How many it walked too, so a check that finds none knows it
        // looked.
        return { walked: seen.size, rows: out, ownVersions: own, copyFirst, skipped,
            uncomposed: loose, composed: composed };
    }

    // Whether Return on a row steps in where the row has an act of its own
    // it would otherwise do, which the step must then lead with: not a
    // copy, which comes after the acts, nor a row that only steps in, asks
    // first or asks for words.
    function stepsOver(row) {
        const verb = verbOf(row).toLowerCase();
        // A row with no act of its own, as a place or a group, steps over
        // nothing: stepping in is all its Return does.
        return !!row.run && stepsIn(row) && !leadsElsewhere(row) && !row.confirm && !row.given
            && !["", "more", "see all", "copy", "close"].includes(verb);
    }

    // Whether a row is an act of copying, as a step's Copy is, rather than
    // a value that copies what it says.
    function copying(row) {
        return !!row && row.figure !== true && !row.answered && verbOf(row) === "copy"
            && /^copy\b/i.test(row.name ?? "");
    }

    // What a row holds as `enter` shows it stepped into: from home or a
    // collection (`things`), what can be done to it, then its parts, a
    // collection's own among them, as a step into it lists them (Agents'
    // Orchestrators, Quests, Runs and AI spend); from a thing's own verbs,
    // what each is done to. One whose rows can't be made out of the
    // launcher's step holds nothing here.
    function catalogueHolds(row, among) {
        try {
            const things = asThing(row, among);
            const making = things ? row.verbs : row.targets;
            const made = making ? making() ?? [] : [];
            const own = things || copiesOnly(row) ? led(row, made) : made;
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
        cataloguing = true;
        try {
            return catalogueRunWalked(id, words);
        } finally {
            cataloguing = false;
        }
    }

    function catalogueRunWalked(id, words) {
        freshen();
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
        const stands = standsOf(category);
        if (stands !== "")
            return stands;
        const names = (category.holds ?? []).map(id => heldName(id)).filter(name => name !== "")
            .concat(commandsIn(category).map(row => row.start.trim()
                || row.key.replace(/^command:/, "")));
        const own = (category.name ?? category.id).toLowerCase();
        return listed(names.filter(name => name.toLowerCase() !== own), room);
    }

    // How a place stands, where the vocabulary names a provider that says
    // so (a category's `stands`): docs/design.md's "a place that measures
    // something says how it stands instead", as Power's row says the charge and the
    // profile where Machine holds it, the way Processor's says how busy it
    // is (2026-10-28: "having power as a top under machine with just a list
    // of options in the hint instead of treated like cpus and stuff is
    // weird"). Asked as its row is made, like the names it replaces; ""
    // when nothing says, and the place lists what it holds instead.
    function standsOf(category) {
        if (!category.stands)
            return "";
        const source = sources.find(one => one.settings.name === category.stands);
        return source?.stands?.() ?? "";
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
            // one feel has it, and the right arrow lands there too, so
            // Return never goes further in than it (Joel, 2026-10-26).
            if (!best.run && (best.verbs || best.targets))
                Object.assign(gathered, {
                    stays: true,
                    lands: true,
                    run: () => root.landInside(gathered, best)
                });
            // The right arrow on any found thing that holds more goes
            // straight inside it, whatever its Return does (VKCF3Z,
            // 2026-10-29: "← is where you came from; the crumbs are
            // where it lives").
            else if (best.verbs || best.targets)
                gathered.landsIn = best;
            rows.push(gathered);
        }
        return rows;
    }

    // Steps into a collection's matches, and then into the one of them that
    // was found, which stepping in keeps selected.
    function landInside(gathered, best) {
        if (enter(gathered) && selected && steadyId(selected) === steadyId(best)
            && enter(selected))
            // The left arrow comes back past the collection to the search.
            held[held.length - 1].through = true;
    }

    // The right arrow: into the selected row, or, for a match that lands
    // inside what it found on Return, there as well, and for a row whose
    // Return goes elsewhere in the launcher, there too.
    function inward(row) {
        if (row && row.lands)
            row.run();
        else if (row && row.landsIn)
            landInside(row, row.landsIn);
        else if (leadsElsewhere(row))
            run(row, false, false);
        else
            enter(row);
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
    // step. Tab is said in the field, at its right edge while there's a
    // completion to take (Prompt.qml), not here: it comes and goes with
    // each letter, so in the foot it moved the keys or took room from how
    // things stand. The left arrow is said only where it goes back: that it
    // takes away what's typed is what any field does, and saying so took
    // room from how things stand as soon as Joel typed. Shift+Return and
    // the right arrow are said wherever they work, a step in too (Joel,
    // 2026-10-26: "the foot shows every key that works on the selection"):
    // the right arrow where `enter` finds somewhere to go, by what it would
    // ask, never by asking it, since the foot is said for every row.
    // Whether Return does something and keeps the launcher, as a setting
    // does, rather than going somewhere inside it, as a spot, a row with
    // more inside, one holding rows (its verbs and targets both, as a
    // meeting's See It) or a collection's match does.
    function keepsOpen(choice) {
        return !!choice.stays && !!choice.run && !choice.spot && !choice.from
            && verbOf(choice) !== "more" && !(choice.verbs && choice.targets)
            && !copiesOnly(choice);
    }

    // Whether a row's only act is copying the figure it shows while it
    // holds more, as a disk's Space does, or an answer listed in its
    // place by name, as Power's Battery is: Return steps in, as the right
    // arrow does, and the copy is listed inside after what can be done
    // (Catalogue.js's leading), so Return never copies and closes where
    // the right arrow would open (Joel, 2026-10-28: "hitting enter on the
    // battery closes the window when right opens it"). An answer to what
    // was typed, a sum or a lookup, still copies on Return, since what was
    // asked is the answer itself; its copy inside comes after its acts too.
    function copiesOnly(choice) {
        return !!choice && (choice.figure === true || (choice.answered === true
            && (choice.placed ?? "") !== "")) && verbOf(choice) === "copy" && holdsMore(choice);
    }

    // Whether a row holds a thing's insides (its verbs, Return's act first,
    // then its parts) rather than a verb's targets: wherever things are
    // listed, at the top or in a collection, and anywhere for a row with
    // verbs and no targets, so a row's insides don't depend on the level
    // it's shown at (Joel, 2026-10-26: one thing, one set of insides). Inside
    // a program, its Shortcuts and its windows held nothing.
    function asThing(row, among) {
        return among || (!row.targets && !!row.verbs);
    }

    // Whether the right arrow steps into a row, as `enter` would: a thing's
    // insides, a verb's targets in a step, a place found from the top, or an
    // ask.
    function holdsMore(choice) {
        if (held.length > 0 && choice.given)
            return true;
        if (held.length === 0 && choice.category)
            return true;
        return asThing(choice, held.length === 0 || collecting) ? !!choice.verbs
            : !!choice.targets;
    }

    // Whether a row with nothing inside it goes somewhere else in the
    // launcher on Return: a spot to where it is, a command into the field
    // to be finished, one that opens a collection into it. The right arrow
    // goes there as well (Joel, 2026-10-26: where Return goes somewhere in
    // the launcher, the right arrow does the same).
    function leadsElsewhere(choice) {
        return !!choice && !choice.gone && !holdsMore(choice)
            && (choice.trail !== undefined || choice.start !== undefined || !!choice.opens);
    }

    // Whether Return steps into a row where the right arrow would, as `run`
    // decides: a place or a holder, a verb wanting a target, one that can't
    // be taken back, or one with nothing to do of its own. Such a row says
    // both keys as one, with the right arrow's word, so stepping in has one
    // word wherever it's done (Joel, 2026-10-26), where it was open, choose,
    // go, see or the act asked about, and never says it twice.
    function stepsIn(choice) {
        if (leadsElsewhere(choice) || copiesOnly(choice))
            return true;
        if (choice.start !== undefined || choice.given || !holdsMore(choice))
            return false;
        if (choice.category || choice.group || choice.lands || !choice.run)
            return true;
        if (held.length > 0 && choice.targets && choice.verb === "more")
            return true;
        return !!choice.confirm && (held.length === 0 || collecting);
    }

    function keysFor(choice) {
        // A row whose thing has gone does nothing, so it says no key.
        if (!choice || choice.gone)
            return [];
        // The arrows are said by their marks alone, which Joel knows: the
        // right arrow steps in and the left one steps back wherever they
        // work, so their words only took room from the marks beside them
        // (Joel, 2026-10-28: "i want icons to be able to be there as much
        // as possible"). Going elsewhere keeps Return's own word, where it
        // takes him.
        const more = leadsElsewhere(choice) ? verbOf(choice) : "";
        const stepping = stepsIn(choice);
        const keys = [stepping ? ["\u21b5 \u2192", more] : ["\u21b5", verbOf(choice)]];
        // One that keeps the launcher can be done and closed at once, the
        // font having no Ctrl key's mark, so it's spelled as chords are.
        if (keepsOpen(choice))
            keys.push(["Ctrl+\u21b5", "close"]);
        if (choice.below)
            keys.push(["\u21e7\u21b5", "below"]);
        if (!stepping && holdsMore(choice))
            keys.push(["\u2192", ""]);
        if (typed === "" && held.length > 0)
            keys.push(["\u2190", ""]);
        // The drawer's keys, only while it holds more than fits, keeping
        // their word as Ctrl+Return does, since a chord isn't an arrow.
        if (choice === selected && drawerOut && drawerCard.more)
            keys.push(["Ctrl+\u2191\u2193", "scroll"]);
        return keys;
    }

    // A key as the foot says it: its mark, then its word where it has one.
    function saidKey(key) {
        return key[1] === "" ? key[0] : `${key[0]} ${key[1]}`;
    }

    // How far in the foot's keys may reach and how wide Return's place is,
    // so they hold still. Said for the selected row alone, the keys moved
    // as the arrows went from a row that opens to one with more behind it,
    // and squeezed how things stand beside them from words to glyphs and
    // back (Joel, 2026-10-25: "jumpy ... at home arrowing around"). So
    // Return, last at the right end, keeps a place as wide as the widest
    // it's been said since the launcher opened, and how things stand keeps
    // clear of where the widest set of keys has reached. The rest sit just
    // before Return, each as wide as it is: a place kept for every key any
    // row had left holes between them as soon as Joel typed ("the
    // whitespacing ... is all fucked up"), and took room from how things
    // stand that no row used. Both only grow while the
    // launcher's open, and only as the rows change, so nothing gives way
    // under the selection. They open as wide as most rows need, Return's
    // place as wide as "connect" and the keys as wide as a setting's with
    // more behind it, so the first letter typed doesn't move them: grown only as
    // rows came, Return moved in as soon as Joel typed, since home's rows
    // say less than what a letter finds.
    readonly property string footVerbRoom: "connect"
    // A row opened below never stays, so Ctrl+Return's place is the wider
    // of the two it could have.
    readonly property var footKeysRoom: [["\u21b5", footVerbRoom], ["Ctrl+\u21b5", "close"],
        ["\u2192", ""]]
    // Return's place, as wide as the widest Return said since the launcher
    // opened, its marks and its word whole: kept by its word alone, a
    // Return said with the right arrow beside it, as a collection's "see
    // all", was wider than the place and moved its marks along from a
    // category's (Joel, 2026-10-25: "jumpy ... at home arrowing around").
    property real footReturnWidest: 0
    property real footKeysWidest: 0

    function footReset() {
        footReturnWidest = measure.advanceWidth(`\u21b5 ${footVerbRoom}`);
        footKeysWidest = keysWide(footKeysRoom);
    }

    // How wide a row's keys are, Return in its place.
    function keysWide(told) {
        if (told.length === 0)
            return 0;
        return told.slice(1).reduce((sum, key) => sum + keys.spacing
            + measure.advanceWidth(saidKey(key)), returnPlace(told[0]));
    }

    // How wide Return's place is beside what it says now.
    function returnPlace(key) {
        return Math.max(measure.advanceWidth(saidKey(key)), footReturnWidest);
    }

    function noteKeys() {
        const rows = asking ? matches.concat([asking]) : matches;
        const said = rows.map(row => keysFor(row)).filter(keys => keys.length > 0);
        const place = Math.max(footReturnWidest,
            ...said.map(keys => measure.advanceWidth(saidKey(keys[0]))));
        if (place > footReturnWidest)
            footReturnWidest = place;
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
    // The same for what every row says at its right, so where a name is
    // cut holds still as "working" becomes "asks you" and back: worked out
    // afresh, each title's cut crept left and right as its side changed
    // (the launcher audit's finding 31, Joel's yes on 2026-10-26).
    property int sidesWidest: 0

    function noteFigures() {
        const step = `${held.map(each => steadyId(each.row)).join("/")}|${typed}`;
        const said = row => ((held.length === 0 ? kindOf(row) : "")
            || ((row.chord ?? []).length > 0 ? "" : row.side ?? "")).length;
        const now = Math.max(0, ...matches.filter(row => !!row.graph).map(said));
        const sides = Math.max(0, ...matches.map(said));
        if (step !== figuresStep) {
            figuresStep = step;
            figuresWidest = now;
            sidesWidest = sides;
            return;
        }
        if (now > figuresWidest)
            figuresWidest = now;
        if (sides > sidesWidest)
            sidesWidest = sides;
    }

    // The shortest way the foot's facts have had to read since the launcher
    // opened (Standing.js's fittedFrom), which they never read longer than
    // until it closes.
    property int footStage: 0
    // The room each of the foot's facts has needed since the launcher
    // opened, by its label and the way it reads, which it keeps: the
    // battery going from 100% to 99%, or the agents from "10 working" to
    // "9 working", slid every fact after it (the launcher audit's finding
    // 32, Joel's yes on 2026-10-26). Kept for each way, so a fact shrunk
    // to its glyph doesn't keep the room its words took.
    property var factsWidest: ({})

    // A step of its own has notes of its own, so the field's right end
    // takes the room they need afresh (Prompt.qml's steady). Known by the
    // path, since a refresh makes the same steps again.
    readonly property string stepsTaken: held.map(step => steadyId(step.row)).join("/")
    onStepsTakenChanged: prompt.forget()

    // How many times the rows have been made, and how many rows drawn
    // afresh, for the check that a refresh redraws only what changed.
    property int rowsMade: 0
    property int rowsDrawn: 0

    onMatchesChanged: {
        rowsMade += 1;
        noteKeys();
        noteFigures();
    }
    onAskingChanged: noteKeys()

    // A verb as the foot says it, one word (Joel, 2026-10-24: "Switch to
    // it" was too many): a turn by the way it goes ("off"), a verb that's
    // one with the way it goes whole ("log out", "put back", whose first
    // word alone says nothing), anything else by its first word.
    readonly property var phrasal: ["in", "out", "back"]

    function oneWord(verb) {
        const said = verb.split(" ");
        if (said[0] === "turn" && said.length > 1)
            return said[said.length - 1];
        return said.length === 2 && phrasal.includes(said[1]) ? verb : said[0];
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
        // it's the one thing there only he can move; the rest follow by
        // name, and its row, there with nothing in it too, isn't one.
        const lead = source.foremost ?? "";
        // A row folded into a place, as Agents' Start a Session is, does
        // something there rather than being one of what it holds, so home
        // never names it.
        const names = source.everything()
            .filter(one => one.key !== "on:waitingOnYou" && one.folded !== true)
            .map(one => one.name);
        const left = room > 0 ? room - row.name.length : Infinity;
        const fitted = names.length === 0 ? ""
            : listed(names, lead === "" ? left : left - lead.length - " \u00b7 ".length);
        // Where not even the first name fits, as a session's whole quest
        // title doesn't, it says how many it holds, as its own count does,
        // and never that it holds none (Joel, 2026-10-28: "agents shows no
        // sessions when i super space right now").
        const rest = names.length > 0 && fitted === "" ? source.counted() : fitted;
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

    // A row on offer now by its key, as it reads now: in the step held, or
    // at the top. Undefined where it's gone.
    function onOffer(key) {
        return held.length > 0 ? offered.find(row => row.key === key && !row.gone) : keyed[key];
    }

    // The rows shown, said again as they are now (Steady.js), each in its
    // place. One still on offer keeps it as it reads now, though what it
    // says no longer matches what's typed: a Play that Return made a Pause
    // shows Pause, rather than going.
    // The row Return last changed something with, kept among what's found
    // until Joel types or steps again, so Wi-Fi Off that became Wi-Fi On
    // still shows it, though "wifi off" no longer finds it.
    property var acted: null
    function stillActed(found) {
        const row = acted ? onOffer(acted.key) : undefined;
        return row && !found.some(one => one.key === row.key) ? [row] : [];
    }

    function steadied(found) {
        const renamed = matches.filter(row => !row.gone && onOffer(row.key)
            && !found.some(one => one.key === row.key)).map(row => onOffer(row.key));
        return Steady.kept(matches, found.concat(renamed), steadyId);
    }

    // Whether the step held last is a collection, whose members are things
    // with verbs of their own, as at the top; or a row that says its verbs
    // are things too (nested), as a report's rows are, so stepping goes on
    // down as far as it holds anything.
    readonly property bool collecting: held.length > 0 && collects(held[held.length - 1])

    function collects(step) {
        return !!step.row.from || !!step.row.nested;
    }

    // What a row holds, led by Return's own act where it was left out
    // (Catalogue.js), that act's name prepared to be found as a
    // provider's rows are.
    function led(row, made) {
        const own = Catalogue.leading(row, made);
        // Return's act leads, or a copy comes after the acts.
        const added = own === made ? undefined
            : own.find(one => String(one.key).startsWith("return:") && !one.prepared);
        if (added)
            added.prepared = Fuzzy.prepare(added.name);
        return own;
    }

    // What a step into a row lists of its own: a thing's verbs, led by
    // Return's act, or a verb's targets; null where it holds none. Made the
    // same way when a step is made again, so a refresh never finds the
    // act it led with gone.
    function stepRows(row, things) {
        const made = things ? (row.verbs ? row.verbs() : null) : row.targets ? row.targets() : null;
        // A row that would only copy gains its copy inside wherever it's
        // stepped into, so the copy is never lost to Return stepping in.
        return made && (things || copiesOnly(row)) ? led(row, made) : made;
    }

    // Steps into a row: a thing's verbs, or a verb's targets. Says whether
    // there was anywhere to go.
    function enter(row) {
        if (!row || row.gone)
            return false;
        // A step is timed to its frame as a key is, named by the arrow and
        // the row, so a slow place shows in the journal and the trials
        // (Joel, 2026-10-28: "programs from the main launcher screen takes
        // for fucking ever to load").
        const began = Date.now();
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
        const things = asThing(row, held.length === 0 || collecting);
        const own = stepRows(row, things);
        // A thing's parts come after what can be done to it.
        const parts = things ? partsIn(row) : [];
        const into = parts.length > 0 ? (own ?? []).concat(parts) : own;
        // A collection with nothing in it still holds the places its
        // vocabulary names, which a step into it shows: Agents with no
        // agent running still leads to its Quests and Orchestrators. A row
        // saying what it holds when empty, as Waiting on You does, is a
        // place stepped into however little it holds.
        const places = things && row.from ? partsOf(row.from).length : 0;
        if ((!into || into.length === 0) && places === 0 && !row.empty)
            return false;
        const made = Date.now();
        held = held.concat([Object.assign(here(), { row: row, rows: into ?? [] })]);
        const pushed = Date.now();
        if (row.from)
            row.from.opened();
        const opened = Date.now();
        // What was typed found the row, so it goes: inside, everything the row
        // holds shows, and the left arrow puts it back on the way out. Stepped
        // into from a collection's match, the match stays selected among all
        // it holds (Joel, 2026-10-26: stepping in keeps the match).
        restart("");
        if (row.from) {
            const matched = matches.findIndex(one => steadyId(one) === steadyId(row));
            if (matched >= 0) {
                current = matched;
                list.positionViewAtIndex(matched, ListView.Contain);
            }
        }
        // Said as a key's are: making the step's lists, what each part of
        // stepping took, and the search that draws its rows.
        searchParts = { lists: made - began, rows: Date.now() - opened, answers: 0,
            each: { holding: pushed - made, opening: opened - pushed } };
        keyTimed(`\u2192 ${row.name}`, began);
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
    // Taken once the change has settled, since a provider may say it
    // changed from within its own things changing (Session.qml does),
    // before the rows read from them (choices, partsOfRows) have heard;
    // each source heard is made again once, in the order heard.
    property var refreshing: []
    function refreshed(source) {
        // A collection's note is read as the corpus is joined, so a source
        // saying it changed marks it stale even when its things are the
        // same.
        choicesStale = true;
        if (refreshing.includes(source))
            return;
        refreshing = refreshing.concat([source]);
        Qt.callLater(refreshDue);
    }
    function refreshDue() {
        if (open)
            freshen();
        const due = refreshing;
        refreshing = [];
        for (const source of due)
            refreshNow(source);
    }
    function refreshNow(source) {
        if (!open || asking)
            return;
        // Out at the top, a row the search found from it is said again in
        // place, so a setting Return changed shows how it stands now
        // (Joel, 2026-10-26: an act that changes something stays open and
        // shows the new state on its row at once).
        if (held.length === 0) {
            if (typed.trim() !== ""
                && matches.some(row => source.things.some(own => own.key === row.key)))
                reshow(0);
            return;
        }
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
            const things = !!thing && asThing(thing, at === 0 || collects(steps[at - 1]));
            const own = !thing ? null : stepRows(thing, things);
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
        root.showMoved(at);
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
        // A thing landed in from a search goes back to the search, past the
        // collection it was found in.
        const steps = held[held.length - 1].through && held.length > 1 ? 2 : 1;
        const left = held[held.length - steps];
        held = held.slice(0, -steps);
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
        if (acted && (letters !== acted.typed || held.length !== acted.depth))
            acted = null;
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
            // A step that makes rows of what's typed (`typed`), as Move the
            // Meeting makes a time, lists them first.
            else if (words.length === 0)
                found = offered.slice();
            else
                found = (held[held.length - 1].row.typed?.(letters) ?? [])
                    .concat(within(offered.reduce((all, row) => all.concat([row],
                        row.offers && row.verbs ? row.verbs() : []), []), letters));
            found = found.concat(stillActed(found));
            answer = null;
            // Whether the drawer is out is the level's, read from all of
            // it as it's stepped into, so narrowing it by typing never
            // slides the drawer in or out.
            if (words.length === 0)
                levelHasDrawer = found.some(row => hasDrawer(row));
            // Searched again only to say what's there now, it keeps the
            // order it's shown in, and the selection where it is (reshow).
            if (steadying) {
                matches = steadied(found);
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
            // Said again in place, what was found is each row as it is now.
            const looked = !narrowing ? choices : !steadying ? candidates
                : candidates.map(choice => onOffer(choice.key)).filter(choice => !!choice);
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
        list.pick(list.settled);
        if (current >= 0)
            list.reveal(current);
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

    // Acting on a thing says it was what was meant by what was typed; in a
    // collection or a category, the one picked from it was. Learned before
    // the launcher closes, while what's typed and held is still there.
    function learn(choice) {
        trials.picked(choice, typed, held.length);
        const first = held.length > 0 ? held[0] : { row: choice, typed: typed };
        const picking = !!first.row.from || !!first.row.category;
        const meant = picking ? choice : first.row;
        // One that can't be taken back is latched only once its first word's
        // been typed (Rank.js's leastOf), so a slip of two letters never
        // powers off.
        picks.picked(picking ? typed : first.typed, meant.key, knowing());
    }

    function run(choice, below, closing) {
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
        // asks for them. A row with an act of its own besides stepping in
        // does it, as a meeting's row joins it (WhatsOn.qml), and the
        // right arrow still finds what it holds.
        if (held.length > 0 && choice.targets && (!choice.run || choice.verb === "more")) {
            enter(choice);
            return;
        }
        if (choice.given) {
            ask(choice);
            return;
        }
        // One that would only copy what it says steps into what it holds.
        if (copiesOnly(choice) && enter(choice))
            return;
        // One that can't be taken back, such as powering off, steps into
        // its verbs, which say what it does, so a second Return does it and
        // Escape or the left arrow backs out.
        if (choice.confirm && (held.length === 0 || collecting) && enter(choice))
            return;
        // One with nothing to do of its own, only more inside, steps in.
        if (!choice.run && enter(choice))
            return;
        // One that changes something, or whose outcome takes a while, keeps
        // the launcher, its row saying how it stands now; Ctrl+Return does
        // it and closes anyway.
        if (choice.stays) {
            const acting = keepsOpen(choice);
            if (acting) {
                learn(choice);
                acted = { key: choice.key, typed: typed, depth: held.length };
            }
            choice.run();
            if (closing && acting)
                open = false;
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
        learn(choice);
        // Said as the foot's other words are, beside the key they follow.
        if (verbOf(choice) === "copy")
            leaveSaying("copied");
        open = false;
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

    // A copy closes with nothing else to show it worked, so the card
    // stays drawn a moment past closing, its foot saying "copied" where
    // Return's key was, then leaves as anything leaving does (Joel,
    // 2026-10-26: "copied" as it closes). The keys go back to the window
    // at once, so nothing waits on it.
    property string parting: ""
    property bool fading: false
    function leaveSaying(word) {
        parting = word;
        fading = false;
        partingLater.restart();
        Report.say(`parted ${word}`);
    }

    // Long enough to read one word, short of where it would linger.
    Timer {
        id: partingLater
        interval: 600
        onTriggered: {
            root.fading = true;
            root.parting = "";
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

        // Every row Return does nothing on, as JSON, for the check.
        function actless(): string {
            return JSON.stringify(root.actless());
        }

        // How many times the search's rows have been joined, for the check
        // that a closed launcher makes nothing of what changes.
        function joined(): string {
            return String(root.choicesJoined);
        }

        // How many frames the card has drawn since this was first asked,
        // for the check that a launcher left still draws nothing.
        function frames(): string {
            root.framesCounted = true;
            return String(root.framesDrawn);
        }
    }

    // Frames drawn, counted only once a check asks, so a launcher in use
    // pays nothing for it.
    property bool framesCounted: false
    property int framesDrawn: 0

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
            // How wide it's drawn and whether it's cut, so a test can say
            // where a name is cut held still. A word in a box that clips it,
            // as a selected name reading on is, is cut by that box rather
            // than by an ellipsis, and the box is the room it holds.
            const clipper = item.parent && item.parent.clip ? item.parent : item;
            const box = clipper.mapToItem(into, 0, 0);
            words.push({ text: item.text, x: Math.round(at.x), y: Math.round(at.y),
                starts: Math.round(at.x + inset), width: Math.round(item.width),
                inked: Math.round(item.contentWidth), cut: item.truncated === true
                    || (clipper !== item && item.implicitWidth > clipper.width),
                room: { x: Math.round(box.x), width: Math.round(clipper.width) } });
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
        // Where the selection is drawn, and where its row is, so a test can
        // say it's drawn on its row.
        const chosen = list.itemAtIndex(root.current);
        const selection = !picked.visible || !chosen ? null : {
            y: Math.round(picked.mapToItem(card, 0, 0).y),
            row: Math.round(chosen.mapToItem(card, 0, chosen.height - Theme.space.xxxl).y)
        };
        return JSON.stringify({ typed: root.typed, field: wordsIn(prompt, card),
            foot: wordsIn(foot, card), rows, selection, made: root.rowsMade,
            drawn: root.rowsDrawn });
    }

    // The drawer beside the card (Drawer.qml): what the selected row shows
    // beyond its line, its own or one made of the picture and the whole
    // words it has. It's out only inside a place, never for a search typed
    // at home, where three letters and Return is the habit and a drawer at
    // every query would be noise; and out for the whole level while any of
    // its rows has one, so it slides only as Joel steps in or out, never as
    // he moves. A row of such a level with none of its own shows its name
    // and note there.
    function drawerOf(row) {
        if (!row)
            return null;
        const own = typeof row.drawer === "function" ? row.drawer() : row.drawer;
        if (own)
            return own;
        if ((row.picture ?? "") === "" && (row.reads ?? "") === "")
            return null;
        return { title: row.name, body: row.reads ?? "", picture: row.picture ?? "",
            across: row.across ?? 0, down: row.down ?? 0 };
    }
    function hasDrawer(row) {
        return !!row.drawer || (row.picture ?? "") !== "" || (row.reads ?? "") !== "";
    }
    property bool levelHasDrawer: false
    // Typed to nothing, it goes in rather than stay out empty or keep the
    // last row's details.
    readonly property bool drawerOut: open && held.length > 0 && levelHasDrawer && !!selected
    // A row with nothing more to show, in a level where the drawer is out,
    // shows its own name alone (VKCF3Z): never the last row's details, never
    // an empty slide-in, and never its note, which the foot already says.
    readonly property var drawerShows: !drawerOut || !selected ? null
        : drawerOf(selected) ?? { title: selected.name }

    // The drawer's room beside the card, and where the card sits (VKCF3Z's
    // ruling, after Joel's "never passes the bound of the screen and
    // resizes to fit there"): the card stays centred, where Super+Space has
    // always opened it, and the drawer takes what's left on its right, up
    // to drawerMost. Only where that's less than drawerLeast, too narrow to
    // hold a line of mail, does the card sit left of centre, the drawer
    // given drawerLeast or what the screen has. Both are worked out for the
    // screen alone, never for what's shown, so the card never moves.
    readonly property int drawerMost: 520
    readonly property int drawerLeast: 280
    readonly property real drawerGap: Theme.space.m
    readonly property real centred: !screenHere ? 0 : (screenHere.width - width) / 2
    readonly property real drawerRoom: !screenHere ? 0
        : screenHere.width - centred - width - drawerGap - Theme.space.xl
    readonly property real drawerWidth: !screenHere ? 0
        : drawerRoom >= drawerLeast ? Math.min(drawerMost, drawerRoom)
        : Math.min(drawerLeast,
            Math.max(0, screenHere.width - width - drawerGap - 2 * Theme.space.xl))
    readonly property real cardLeft: !screenHere ? 0 : Math.max(Theme.space.xl,
        Math.min(centred, screenHere.width - Theme.space.xl - drawerWidth - drawerGap - width))

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
            // What the list says in place of rows, where it has none.
            nothing: nothingShown.visible ? nothingShown.name : "",
            selected: !row ? null : {
                name: row.name,
                is: Rank.identity(row),
                verb: root.verbOf(row),
                chord: (row.chord ?? []).map(key => key.text).join(" "),
                side: row.side ?? "",
                // What's said below its name, a collection's note.
                note: row.note ?? "",
                // The whole words drawn below the rows for it, if any.
                reads: row.reads ?? "",
                // What the drawer beside the rows shows of it, if anything.
                drawer: row.drawer ?? null,
                // The keys the foot says for it, each a mark and its word.
                told: root.keysFor(row).map(key => root.saidKey(key)),
                settled: !(row === root.headline || row.answered)
                    || !(root.answer && root.answer.asking)
            },
            kind: root.kind,
            footer: foot.said,
            glance: standing.said,
            // Where Return's key starts, which holds still as the arrows go.
            keysAt: Math.round(foot.returnAt),
            // How far the list is scrolled, which only a key changes.
            scrolled: Math.round(list.contentY),
            // The drawer: whether it's out, what it's titled and says,
            // whether it holds more than fits, and where it and the card sit.
            drawer: {
                out: root.drawerOut,
                title: root.drawerShows?.title ?? "",
                facts: (root.drawerShows?.facts ?? []).filter(fact => fact)
                    .map(fact => fact.label ?? ""),
                body: root.drawerShows?.body ?? "",
                more: drawerCard.more,
                scrolled: Math.round(drawerCard.scrolled),
                cardAt: Math.round(card.mapToGlobal(0, 0).x),
                at: Math.round(drawerCard.mapToGlobal(0, 0).x),
                width: Math.round(drawerCard.width),
                height: Math.round(drawerCard.height),
                cardHeight: Math.round(card.height)
            }
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
    // has opens the search.
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
        // Every place opens on its first row, the same each time, so Joel
        // knows where the keys are before he looks (VKCF3Z, 2026-10-29). A
        // collection may lead into one of its parts, as Keyboard does into
        // the program in use's keys, which opens on its first row too.
        const lead = source.leading ? source.leading() : null;
        if (lead)
            enter(lead);
    }

    onOpenChanged: {
        if (open) {
            freshen();
            trials.opened();
            partingLater.stop();
            fading = false;
            parting = "";
            footReset();
            footStage = 0;
            factsWidest = {};
            figuresStep = "";
            held = [];
            prompt.forget();
            often = picks.counts();
            spotsLater.stop();
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
            spotsLater.restart();
        }
    }

    // What's inside things is read again a moment after the launcher
    // closes, from what it just showed, so the next opening has it
    // ready. Read on the third letter instead, as it was, it held that
    // key 115 to 250 ms once every opening (launcher-trials.json,
    // 2026-10-26: "wha" 252 ms, "cpu" 248, "key" 211). The first opening
    // after the shell starts still reads it on its third letter.
    Timer {
        id: spotsLater

        interval: 500
        onTriggered: {
            if (!root.open) {
                root.freshen();
                root.spots = root.spotsFrom();
                root.spotsMade = true;
            }
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

    // The card and the drawer out beside it, for the pointer and the blur.
    Region {
        id: cardAndDrawer
        regions: [card.area, drawerCard.area]
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
        // Placed from the left, so the card sits where cardLeft says with
        // the drawer's room on its right, sized once for the screen.
        anchors.top: true
        anchors.left: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: root.open && screen // qmllint disable unqualified
            ? screen.height / 4 - card.reach : 0 // qmllint disable unqualified
        margins.left: root.open ? root.cardLeft - card.reach : 0 // qmllint disable unqualified
        implicitWidth: root.open
            ? root.width + root.drawerGap + root.drawerWidth + 2 * card.reach : 1
        implicitHeight: root.open ? card.height + 2 * card.reach : 1
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: !root.open ? nowhere : root.drawerOut ? cardAndDrawer : card.area
        BackgroundEffect.blurRegion: !root.open ? nowhere
            : root.drawerOut ? cardAndDrawer : card.area
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

        Connections {
            target: card.Window.window
            enabled: root.framesCounted

            function onFrameSwapped() {
                root.framesDrawn += 1;
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
            opacity: root.open || root.parting !== "" ? 1 : 0

            // Only a parting word's card leaves on the curve; anything
            // else closes on the next frame, as it opens.
            Behavior on opacity {
                enabled: root.fading
                NumberAnimation {
                    duration: Theme.motion.short
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }

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
                        onAccepted: (below, closing) => {
                            trials.pressed("returns");
                            if (root.asking)
                                root.give();
                            else
                                root.run(root.selected, below, closing);
                        }
                        // Tab on an answer puts the answer in place of the
                        // question, so a sum can be carried on; one still
                        // being asked for leaves the question as it is.
                        onCompleted: {
                            trials.pressed("tabs");
                            // Tab does something only while the field
                            // shows a grey completion, and only takes it;
                            // stepping in is the right arrow's (Joel,
                            // 2026-10-26). At home there's none, since
                            // nothing is typed (2026-10-25: "i don't like
                            // tab for browse, that seems useless").
                            if (root.completion === "")
                                return;
                            // A command puts its start in the field.
                            if (root.current !== -1 && root.selected?.start !== undefined)
                                root.fill(root.selected.start);
                            // A question only started is finished first.
                            else if (root.question !== ""
                                && (root.current === -1 || root.selected?.answered))
                                text = root.question;
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
                            root.inward(root.matches[root.current]);
                        }
                        onBackward: {
                            trials.pressed("backs");
                            root.back();
                        }
                        // The highest the selection goes: an answer's headline
                        // sits above the rows, at -1.
                        readonly property int highest: root.answer && !root.parted(root.answer)
                            ? -1 : 0
                        // With Ctrl, the arrows scroll the drawer instead,
                        // since Page Up and Page Down page the list.
                        Keys.onUpPressed: event => {
                            if (event.modifiers & Qt.ControlModifier) {
                                drawerCard.scroll(-1);
                                return;
                            }
                            trials.pressed("arrows");
                            root.current = Math.max(root.current - 1, highest);
                        }
                        Keys.onDownPressed: event => {
                            if (event.modifiers & Qt.ControlModifier) {
                                drawerCard.scroll(1);
                                return;
                            }
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
                            // And the widest any row has said there.
                            readonly property int sides: root.sidesWidest
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            // The rows by what each is (steadyId), each
                            // drawn once and kept while it's there: a refresh,
                            // each second inside Machine and four times a
                            // second while Sound's meter runs, made every row
                            // afresh, restarting whatever moved on it (the
                            // launcher audit's finding 30, Joel's yes on
                            // 2026-10-26). A row kept reads what it says now
                            // from root.matches by its place.
                            model: ListModel {
                                id: rowsKept
                            }

                            // Brings the kept rows into line with these, by
                            // what each is: those the same at the start and
                            // the end stay, and only those between are drawn
                            // afresh. Says whether they were all the same.
                            // The rows go in between the removing and the
                            // adding, so no kept row ever reads past the end
                            // of the rows it reads: the removing leaves none
                            // past the old ones' end, and the new ones reach
                            // as far as what's left and what's added.
                            function keep(rows) {
                                const keys = rows.map(row => root.steadyId(row));
                                let start = 0;
                                while (start < rowsKept.count && start < keys.length
                                    && rowsKept.get(start).key === keys[start])
                                    start += 1;
                                if (start === rowsKept.count && start === keys.length) {
                                    list.rows = rows;
                                    return true;
                                }
                                let was = rowsKept.count;
                                let now = keys.length;
                                while (was > start && now > start
                                    && rowsKept.get(was - 1).key === keys[now - 1]) {
                                    was -= 1;
                                    now -= 1;
                                }
                                if (was > start)
                                    rowsKept.remove(start, was - start);
                                list.rows = rows;
                                for (let at = start; at < now; at += 1)
                                    rowsKept.insert(at, { key: keys[at] });
                                return false;
                            }

                            // The launcher's one motion is the selection
                            // moving to its row, the list scrolling with it
                            // on the same timing, the theme's medium on its
                            // curve (the launcher audit's finding 33, Joel's
                            // yes on 2026-10-26: it jumped, and the list
                            // snapped a row at a time). Only a move among the
                            // same rows glides: rows made afresh, by a letter,
                            // a step or a refresh, are a new list, and its
                            // selection is where it is at once, so typing
                            // never glides. Everything else stays instant.
                            property bool settled: false
                            readonly property var shown: root.matches
                            property string typedWas: ""
                            // The rows as the kept ones read them, set by
                            // keep partway through, so none reads past the end.
                            property var rows: []
                            onShownChanged: {
                                const same = list.keep(root.matches);
                                // The same rows read again leave a glide under
                                // way; a letter typed never glides, even onto
                                // the same rows.
                                // A glide under way is aimed again where its
                                // row now is, should a row above it have
                                // changed height, so it never ends off it.
                                if (same && root.typed === list.typedWas) {
                                    if (!picking.running)
                                        list.pick(false);
                                    else if (picking.to !== list.fillAt(root.current))
                                        list.pick(true);
                                    return;
                                }
                                list.typedWas = root.typed;
                                list.settled = false;
                                // A glide under way ends where it was going,
                                // never stopped short.
                                scrolling.complete();
                                list.pick(false);
                                Qt.callLater(() => list.settled = true);
                            }

                            // Where the selection is drawn, set by pick
                            // rather than bound, so a glide under way gives
                            // way at once to rows made afresh.
                            property real pickY: 0

                            // Where the fill sits on a row, counted from
                            // the first row's top as rowTop is; the fill
                            // adds the list's origin, below.
                            function fillAt(at) {
                                return list.rowTop(at) + list.rowHeight(at) - Theme.space.xxxl;
                            }

                            function pick(glide) {
                                const to = list.fillAt(root.current);
                                picking.stop();
                                if (glide && to !== list.pickY) {
                                    Report.say(`glides ${Math.round(list.pickY)} to ${Math.round(to)}`);
                                    picking.to = to;
                                    picking.start();
                                } else
                                    list.pickY = to;
                            }

                            NumberAnimation {
                                id: picking
                                target: list
                                property: "pickY"
                                duration: Theme.motion.medium
                                easing.type: Easing.BezierSpline
                                easing.bezierCurve: Theme.motion.curve
                            }

                            // Where a row's own place starts and ends in the
                            // list, its gap above it included.
                            function rowTop(at) {
                                let top = 0;
                                for (let each = 0; each < at && each < root.matches.length; each += 1)
                                    top += Theme.space.xxxl + (root.matches[each].gap ? list.apart : 0);
                                return top;
                            }

                            function rowHeight(at) {
                                return Theme.space.xxxl + (root.matches[at]?.gap ? list.apart : 0);
                            }

                            // Scrolls just far enough to show the row, as
                            // positionViewAtIndex's Contain does, gliding
                            // there when the selection glides.
                            function reveal(at) {
                                if (!list.settled) {
                                    list.positionViewAtIndex(at, ListView.Contain);
                                    return;
                                }
                                // From where a glide under way is going, so a
                                // second key while it runs goes on from there.
                                const from = scrolling.running ? scrolling.to : list.contentY;
                                const top = list.originY + list.rowTop(at);
                                const bottom = top + list.rowHeight(at);
                                const to = top < from ? top
                                    : bottom > from + list.height ? bottom - list.height : from;
                                if (to === from)
                                    return;
                                scrolling.stop();
                                scrolling.to = to;
                                scrolling.start();
                            }

                            NumberAnimation {
                                id: scrolling
                                target: list
                                property: "contentY"
                                duration: Theme.motion.medium
                                easing.type: Easing.BezierSpline
                                easing.bezierCurve: Theme.motion.curve
                            }

                            // The selection itself, under the rows, which
                            // draw no fill of their own here (Choice's fills).
                            Rectangle {
                                id: picked

                                // Among the rows, so it scrolls with them:
                                // a child of the list itself stayed put as
                                // the list scrolled, and sat as far below
                                // its row as the list had gone down.
                                parent: list.contentItem
                                z: -1
                                width: list.width
                                height: Theme.space.xxxl
                                // The rows sit from the list's origin, which
                                // moves off 0 as rows above come or go while
                                // the list is scrolled, so the fill does too,
                                // at once, as its row does. Counted from 0, it
                                // stayed where its row had been, under
                                // another row (Joel, 2026-10-29: the
                                // highlight sat off the selected text).
                                y: list.originY + list.pickY
                                visible: root.current >= 0 && root.current < root.matches.length
                                radius: Theme.inner
                                color: Qt.alpha(Theme.selection, Theme.floating.selection)
                            }

                            delegate: Item {
                                id: entry

                                required property int index
                                // What it says now, read by its place, so a
                                // row kept across a refresh says the new. A
                                // row let go has no place (Qt gives it -1)
                                // until it's destroyed, a moment later, and
                                // says what it last said till then, since
                                // read by a place it hasn't, it said nothing
                                // and every binding on it failed.
                                readonly property var modelData: entry.index >= 0
                                    ? list.rows[entry.index] : entry.said
                                property var said: ({})
                                onModelDataChanged: {
                                    if (entry.index >= 0)
                                        entry.said = entry.modelData;
                                }
                                Component.onCompleted: root.rowsDrawn += 1
                                // What it's inside, whose glyph an act of its
                                // own without one wears.
                                readonly property var thing: root.held.length > 0
                                    ? root.held[root.held.length - 1].row : null
                                // Its own glyph, or its thing's, for an act.
                                readonly property bool own: (modelData.glyph ?? "") !== ""
                                // Holding more (an answer's parts, a row of rows, a
                                // collection, a part whose verb is "more", a figure
                                // that copies and holds more) before doing, and a
                                // figure only once it does nothing else
                                // (Provider.qml's copyRow and holderRow).
                                readonly property bool holds: !!(modelData.opens
                                    || modelData.nested || modelData.verb === "more"
                                    || modelData.figure === true && modelData.targets)
                                readonly property string sort: holds ? "holds"
                                    : modelData.figure === true && !modelData.verbs ? "figure"
                                    : "act"

                                width: list.width
                                height: Theme.space.xxxl + (modelData.gap ? list.apart : 0)

                                Choice {
                                    anchors.bottom: parent.bottom
                                    selected: entry.index === root.current
                                    fills: false
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
                                    sides: list.sides
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
                                }
                            }
                        }

                        // With nothing typed, the shell says what it can do, or an
                        // empty collection says it's empty in its own words; with
                        // nothing found, it says so. A field asking for words has
                        // only space below it.
                        Choice {
                            id: nothingShown

                            readonly property var holder: root.held.length > 0
                                ? root.held[root.held.length - 1].row : null
                            readonly property var inside: holder?.from ?? null
                            visible: root.matches.length === 0 && root.answer === null
                                && root.asking === null
                            glyph: root.typed === "" ? Theme.glyph.greeting
                                : Theme.glyph.nothingFound
                            hue: Theme.voice
                            quiet: true
                            name: root.typed !== ""
                                ? `Nothing answers to \u201c${root.typed}\u201d.`
                                : holder?.empty ? holder.empty
                                : inside?.collection
                                    ? inside.collection.empty ?? `No ${inside.collection.noun} yet.`
                                : "A window, a program, an action, or what a key does."
                        }
                    }

                    // A picture, or a row's whole words, show in the drawer
                    // beside the card, never below the rows, so the card's
                    // height never changes with the selection.

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
                        // A parting word stands where Return was, its
                        // mark in the success colour.
                        readonly property var told: root.parting !== ""
                            ? [[Theme.glyph.done, root.parting, Theme.success]]
                            : root.keysFor(choice)
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
                        // glyph alone, and a glyph never gives way
                        // (launcher/Standing.js). Each keeps the room it
                        // has needed since the launcher opened. Two
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
                            // The room a fact takes: what it says, or the
                            // most it's said that way since the launcher
                            // opened (root.factsWidest).
                            function place(fact) {
                                return Math.max(measure.advanceWidth(fact.said),
                                    root.factsWidest[`${fact.entry.name}:${fact.way}`] ?? 0);
                            }

                            readonly property var fit: {
                                const wide = facts => facts.reduce((sum, fact) => sum + place(fact), 0)
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
                                if (!root.open)
                                    return;
                                const widest = Object.assign({}, root.factsWidest);
                                let grew = false;
                                for (const fact of kept) {
                                    // By its name, which each has and
                                    // none shares: by its label, the time
                                    // and the date, which have none, kept
                                    // one room, and the time kept the
                                    // date's once the date gave way.
                                    const key = `${fact.entry.name}:${fact.way}`;
                                    const now = measure.advanceWidth(fact.said);
                                    if (now > (widest[key] ?? 0)) {
                                        widest[key] = now;
                                        grew = true;
                                    }
                                }
                                if (grew)
                                    root.factsWidest = widest;
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

                                    width: standing.place(modelData)
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
                            // place (footReturnWidest), the rest close by.
                            keys: foot.shown
                            endsRight: true
                            widths: foot.shown.map((key, at) => at === foot.shown.length - 1
                                ? foot.returnPlace : 0)
                        }
                    }
                }
            }
        }

        // Out of the card's right side, top aligned and as tall as it.
        Drawer {
            id: drawerCard
            x: card.reach + root.width + root.drawerGap
            y: card.reach
            width: root.drawerWidth
            tall: card.height
            out: root.drawerOut
            given: root.drawerShows
        }
    }
}
