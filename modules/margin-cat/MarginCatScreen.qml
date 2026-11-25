// One screen's margin cat (MarginCat.qml makes one for each): where on this
// screen it sits, over the column Joel's keys are in, how it walks, hops
// and wanders there and gets about the windows between workspaces, by the
// ways MarginCatPaths.js finds, and how it's drawn, a frame each time the
// screen draws while it moves and none at rest. What it plays, and when, is the
// scene's (MarginCat.qml), which every screen's cat shares, so a stroll is
// taken by one screen alone.
// Long because its place, walks, moves and frames all hang on one surface's
// state as niri says the layout; its ways and their logic are already the
// .js files beside it, tested in Qt's engine.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Wayland
import QtQuick
import KeyTimes
import "MarginCat.js" as MarginCat
import "MarginCatFloats.js" as Floats
import "MarginCatGames.js" as Games
import "MarginCatPaths.js" as Paths
import "MarginCatPlays.js" as Plays
import "MarginCatShown.js" as Shown
import "MarginCatStacked.js" as Stacked
import "MarginCatTiles.js" as Tiles
import "../Pixels.js" as Pixels

PanelWindow {
    id: window

    required property ShellScreen modelData
    // The screen while Quickshell still lists it, and null once it's gone.
    // Qt drops a screen from the list before it frees it, and this surface
    // outlives both a moment, while modelData stays a screen whose every
    // read warns of a dangling object: a niri event then woke the column,
    // the seat and the workspace, which read its name and width
    // (vm-screens-keep-the-launcher, 12 to 27 warnings a run on
    // 2026-11-17). Whatever an event can wake reads this instead, and
    // modelData is read only as the surface's screen and within a walk,
    // whose clocks stop as the screen goes.
    readonly property ShellScreen present: Quickshell.screens.includes(modelData)
        ? modelData : null
    onPresentChanged: {
        if (present !== null)
            return;
        walker.stop();
        mover.stop();
        resting.stop();
        leaving.stop();
    }
    // The screen's name, taken once as the surface is made, since a
    // screen keeps its name: what it says and asks of niri reads this,
    // so neither reads the screen once it's gone.
    readonly property string named: modelData?.name ?? ""
    // The scene every screen's cat shares: how lively it is, the pose it
    // holds, and the stroll or hop under way (MarginCat.qml).
    required property var scene

    // The window the keys are in, where it's on this screen's shown
    // workspace, and the cat's to sit over.
    readonly property var wanted: {
        void window.scene.settled;
        void Niri.revision;
        if (window.scene.shared || Niri.overview || Niri.current === null)
            return null;
        return tile(Niri.current) === null ? null : Niri.current;
    }
    // The window whose column the cat sits over, or last sat over
    // while it walks or waits to: the one the keys rested in for two
    // seconds, so tapping h and l along the columns never sets it
    // walking.
    property var sat: null
    // The column of the window the keys are in, as niri says its tile
    // will come to rest, so it moves with the column as niri lays
    // them out again. The shell hears that tile alone, so where the
    // cat walks from is where it's drawn, never the column it left.
    readonly property var column: {
        void window.scene.settled;
        void Niri.revision;
        return window.scene.shared || Niri.overview ? null : tile(wanted);
    }
    // Each cell in whole device pixels, and the cat's size from them.
    // The window's own ratio, which a fractional scale sets, rather
    // than the screen's, as Hairline.qml reads it. Taken a moment
    // late: showing or hiding the window sets its ratio, so read at
    // once it would change the place that showed it, a loop Joel's
    // shell said 327 times at 1.5 on 2026-11-12 from 15:12 to 15:57,
    // three around each launcher opening.
    property real ratio: cat.Screen.devicePixelRatio > 0
        ? cat.Screen.devicePixelRatio : 1
    Binding on ratio {
        value: cat.Window.window?.devicePixelRatio
            ?? (cat.Screen.devicePixelRatio > 0 ? cat.Screen.devicePixelRatio : 1)
        delayed: true
    }
    readonly property real cell: MarginCat.cell(Theme.gap, ratio) / ratio
    readonly property real size: Math.max(1, window.scene.drawing.length) * cell
    // Where it'd sit over the keys' column, or along the margin from
    // it while a wander has it there, and where it's drawn: there, a
    // step of its walk, or still where it sat while it waits for the
    // keys to rest.
    // It reads nothing follow() writes, since a write there would
    // work it out again inside its own change, a binding loop.
    readonly property var there: column === null ? null
        : strolled ?? seat(column)
    // Whether the workspace this screen shows has no windows, where it
    // has no seat but the screen's floor: it falls there and sits, and
    // climbs back by an edge as windows come (VKCF3Z, 2026-11-14,
    // e5a3ee44). Never in the overview or a shared screen, where it
    // goes as ever.
    readonly property bool emptied: {
        void Niri.revision;
        return space !== null && !window.scene.shared && !Niri.overview
            && Tiles.empty(Niri.windows, space.id);
    }
    onEmptiedChanged: {
        if (emptied)
            follow();
    }
    // A stroll's: the seat it wanders to or sits at, or null for
    // its column; whether the walk under way is a scene's, which
    // hands the scene its next beat as it sits; and whether the next
    // move of its seat walks rather than carries it there, hurried
    // (true, a key bringing it back) or not (false), or null for a
    // column moving, which carries it.
    property var strolled: null
    property bool handsBack: false
    property var roams: null
    property var place: null
    // The column it last sat over or set off for, so its window grown,
    // shrunk or moved up or down under it walks it rather than carries
    // it (follow), the windows as follow() last saw them, and as they
    // last held the window it stands on (Tiles.before).
    property var under: null
    property var seen: []
    property var seenOn: []
    // The window it sat on that closed, when, and the windows as they
    // held it, while nothing has moved yet: niri first puts the row
    // below in the closed one's place, its windows not resized yet, and
    // only then lays the column out again, so the fall waits for that
    // (the GPU rig, 2026-11-15: it rode the row below down instead,
    // sitting, a jump of 105 px).
    property var closedOn: null
    property real closedAt: 0
    property var closedSaw: null
    // The walk under way, the window it walks to, and the step it's
    // on: its pose, by the theme's name, and the way it faces.
    property var going: null
    property var goingTo: null
    // The window it waits on the keys to rest in.
    property var waitingOn: null
    property var stepped: null
    // Its pose and the way it faces, set only as they change, so
    // its cells are laid again a few times a walk rather than on
    // every frame, which dropped five of a walk's frames on a
    // builder.
    property string pose: ""
    property int facing: 0
    // Whether it's sitting still, neither going, moving, riding its
    // window's scroll, at play nor stretching, so a pointer may pet it and
    // a play may set off from where it's drawn; and since when, so one it
    // came to under isn't one that moved onto it (MarginCat.strokes).
    readonly property bool still: going === null && moving === null && !holding
        && carry === null
        && (yarnAt === null || yarnLies) && window.scene.posed === "" && !window.scene.stretching
    property real stillSince: -1
    onStillChanged: {
        if (still) {
            rests();
        } else {
            satAt = -1;
            stillSince = -1;
            unpetted();
        }
    }
    // Its rest begins once niri has answered for the first frame synced
    // after it sat, which holds its cells where it sits, and counts from
    // when the shell heard that answer, on the monotonic clock. niri sends
    // the pointer over its cells as it takes the commit moving them,
    // before it answers for that frame, so however long niri was held up,
    // a pointer it came to under is felt before its rest begins, or as
    // soon after as the shell hands that enter on: 46.7 ms once in 12 on an
    // idle builder, and never under load (2026-11-24), where SETTLED gives
    // it 200. Counted from its walk's end, a builder under load held that
    // commit up to 233 ms after it sat and the cat leant on a pointer it
    // had walked under. An ask only marks the frame Qt draws for its
    // sitting pose, drawing none of its own, but for when no frame was
    // coming: a glance still draws two (MarginCat.restFrom).
    property real satAt: -1
    property int satFrom: 0
    function rests(): void {
        stillSince = -1;
        if (!window.FramePresentation.available) {
            stillSince = KeyTimes.now();
            return;
        }
        satAt = KeyTimes.now();
        satFrom = window.FramePresentation.framesBegun;
        window.FramePresentation.ask();
    }
    // A frame niri showed or let go, `frame`, as MarginCat.restFrom says.
    function restSeen(frame: int): void {
        if (satAt < 0)
            return;
        const seen = MarginCat.restFrom(frame, window.FramePresentation.timesOf(frame).synchronizing,
            satFrom, satAt);
        if (seen === "begins") {
            satAt = -1;
            stillSince = KeyTimes.now();
        } else if (seen === "asks") {
            window.FramePresentation.ask();
        }
    }
    // Where the pointer was last seen over it, in the screen's pixels,
    // null with none; the way it leans into it, 0 while it doesn't
    // (MarginCat.leansTo); and the way it'd lean as the pet comes.
    property var pointer: null
    property int leaning: 0
    property int leaningTo: 0
    onLeaningChanged: say()
    // Where across its cells the pointer was last seen, so a swat after
    // one crossing it reaches toward the side it left by; the way its paw
    // is out, 0 while it isn't (MarginCat.swatting); and when it last was.
    property real across: 0
    property int swatting: 0
    property real swattedAt: 0
    onSwattingChanged: say()
    // When niri showed each frame of the walk under way, how long the
    // shell took to draw each, and how many niri never showed, said
    // once it sits (Shown.walked). Kept as plain arrays, pushed
    // to, so a frame's answer notifies nothing.
    property var shown: []
    property var costs: []
    property int unseen: 0
    // How many frames had begun as it set off, so it can say how
    // many it drew, and how many steps it has taken since, so it can
    // say whether it moved on each.
    property int begunAt: 0
    property int steps: 0
    // A walk, move or play done but not yet told, its kind, while niri
    // still answers for the frames on their way as it sat: the last of
    // them, how many it began and how many steps it took.
    property string telling: ""
    property int tellsAt: 0
    property int tellsBegun: 0
    property int tellsStepped: 0
    // How it was held under the launcher while it went (stilled): the
    // frames its own surface began meanwhile, none of them steps of its
    // walk, left out of the walk's count; where the hold began; and each
    // frame in `shown` that came first after one, so the journal leaves
    // the hold out of its gaps. `heldAt` is -1 while no hold is open.
    property int heldBegun: 0
    property int heldAt: -1
    property var resumedAt: []
    function unheld(): void {
        heldBegun = 0;
        heldAt = -1;
        resumedAt = [];
    }
    // Whether what it plays was set off by Joel's hand: a drop, a fetch or
    // a chase, or the yarn fading as it's picked up. His hand is the input
    // then, and the card covers none of it, so it's never held, lest it
    // hang where he let it go till the card goes.
    property bool byHand: false
    // How often this screen draws, in ms, as its last walk showed.
    property real frameMs: 1000 / 60
    // The workspace this screen shows, its id and index, so a move
    // between two has it hold its ground through the slide and walk to
    // its new seat (VKCF3Z, 074321bd, 4d18ade4), and the one it showed.
    readonly property var space: {
        void Niri.revision;
        const name = present?.name ?? "";
        const shows = Object.values(Niri.workspaces)
            .find(workspace => workspace.output === name && workspace.is_active);
        return shows ? { id: shows.id, idx: shows.idx } : null;
    }
    property var spaceWas: null
    // The workspace a move between them left, whose windows it walks off
    // going down (Stacked.moves).
    property var spaceLeft: null
    // The move between workspaces under way (Stacked.moves), the window
    // it goes to once niri says it, and its quarter turn as it climbs.
    property var moving: null
    property var movingTo: null
    property int turn: 0
    // The margin's marks, where they're drawn (Paths.marks), which it
    // hops across one at a time, perched on each.
    readonly property var marksLit: present === null ? null
        : Paths.marks(MarginMarks.drawn, present.width, Theme.gap)
    // What it's drawn as: a pose of its walk, turned the way it
    // goes and the way it climbs, leaning into a pointer, or the
    // scope's, sitting, asleep or stretching, or swatting after one;
    // in Joel's hand and let go of, as MarginCat.fromHand says.
    readonly property string handed: MarginCat.fromHand(holding, going?.kind === "drop",
        stepped) ?? ""
    readonly property var drawn: handed === "scruffed" ? Theme.pixels.scruffed
        : handed ? MarginCat.turned(Theme.pixels[handed], facing)
        : swatting !== 0 ? MarginCat.swatting(swatting, Theme.pixels)
        : leaning !== 0 ? MarginCat.leaning(leaning, Theme.pixels)
        : pose ? MarginCat.quartered(
            MarginCat.turned(Theme.pixels[pose] ?? window.scene.drawing, facing), turn)
        : window.scene.drawing
    // Whether it wants every tile it crosses (MarginCat.travels), and as
    // which it wants them.
    readonly property bool travels: MarginCat.travels({
        going: going !== null,
        handsBack,
        moving: moving !== null,
        resting: resting.running,
        holding
    })
    property string wantsAs: ""
    // Wanted once the change is done: niri's windows notify as they're
    // wanted, and set it going or moving again inside the change, a
    // binding loop (the GPU rig, 2026-11-13).
    onTravelsChanged: Qt.callLater(wantsTiles)
    // Set as it's destroyed, so a want called later wants nothing.
    property bool gone: false
    function wantsTiles(): void {
        if (gone)
            return;
        if (travels && wantsAs === "") {
            wantsAs = `cat ${named}`;
            Niri.wantTiles(wantsAs, true);
        } else if (!travels && wantsAs !== "") {
            Niri.wantTiles(wantsAs, false);
            wantsAs = "";
        }
    }

    // Where the keys' column now has it: nowhere, once it's held
    // nowhere a moment (leaving); from
    // nowhere, there at once, since nothing walks in from nowhere;
    // over the same window, carried as its column scrolls, or, its
    // window grown, shrunk or moved up or down under it, on its way
    // there from where it is, as it is where its seat moved a pixel on
    // the way there; and over another, where it is or on to where its
    // walk was going, until the keys rest there two seconds, never
    // put anywhere it didn't get to (WCJQSG 2bd7c7c6). niri says the
    // layout again for any window's change, a title's too, so the same
    // seat neither sets a walk off again nor restarts its rest.
    function follow(): void {
        const saw = Tiles.before(seen, seenOn, standsOn());
        seen = tilesOf(space?.id);
        if (seen.some(one => one.id === standsOn()))
            seenOn = seen;
        // Held, it goes nowhere the keys say, and stays where his hand
        // has it (VKCF3Z, 9ead6710).
        if (holding)
            return;
        // The window it was let go onto gone, it falls again from there.
        if (going === null && landedOn?.id !== null && landedOn?.id !== undefined
            && Niri.windows[landedOn.id] === undefined) {
            drops();
            return;
        }
        if (moving !== null) {
            steer();
            return;
        }
        if (there === null) {
            if (emptied) {
                leaving.stop();
                floors();
            } else if (place !== null) {
                leaving.restart();
            }
            return;
        }
        leaving.stop();
        if (place !== null && going === null && wanted === sat && roams !== null) {
            const hurried = roams;
            roams = null;
            resting.stop();
            walkFrom(hurried, saw);
        } else if (place !== null && standsOn() !== wanted && standsOn() !== null
            && Niri.windows[standsOn()] === undefined) {
            // The window it sat on, or rode, closed under it, which niri
            // takes away at once: it gets off now, never waiting for the
            // keys to rest (VKCF3Z 26038726), and a row of a column falls
            // ahead of the window grown down over it (Joel, 2026-11-14
            // 17:24: "it seems to hang out in the middle then walk
            // across, that is not believable"). niri moves a closing
            // window's tile before it says the window's gone, which sets
            // a ride going first (a nested niri, 2026-11-14).
            resting.stop();
            closedOn = standsOn();
            closedAt = Date.now();
            closedSaw = saw;
            walkFrom(true, saw);
        } else if (place === null || (going === null && wanted === sat)) {
            resting.stop();
            // A floating window moved up or down carries it, as it's
            // dragged; only one grown or shrunk across walks it.
            const reshaped = place !== null && wanted === sat && under !== null
                && column !== null && (Math.abs(column.width - under.width) > 0.5
                    || (!column.floats && Math.abs(column.top - under.top) > 0.5));
            sat = wanted;
            under = column;
            if (reshaped) {
                walkFrom(true, saw);
            } else {
                // Back on its own column, it rides with its window from
                // where it's drawn, sits there at once, or lets a ride
                // under way run on (Stacked.backOn).
                const back = Stacked.backOn(place, there, {
                    wide: size,
                    across: modelData.width,
                    riding: carry !== null,
                    scrolling: scrolling()
                });
                if (back === "rides")
                    rideTo(there);
                else if (back === "sits")
                    place = there;
            }
        } else if (going !== null && wanted === goingTo) {
            if (Math.round(there.x * ratio) !== Math.round(going.to.x * ratio)
                || Math.round(there.y * ratio) !== Math.round(going.to.y * ratio))
                walkFrom(true, saw);
        } else {
            // Walking still to a window the keys have left, as niri
            // scrolls the strip under it, it stops where it's drawn and
            // rides that window, as it would sitting, rather than walking
            // on to where that window was, over the air. A drop by his
            // hand falls on as it was let go.
            if (going !== null && !byHand
                && MarginCat.carries(saw, tilesOf(space?.id), goingTo) !== null) {
                const was = goingTo;
                halt();
                sat = was;
            }
            if (going === null)
                carried(saw);
            if (going !== null || !resting.running || wanted !== waitingOn) {
                // Its wander forgotten a moment later, outside this
                // change of `there`, which forgetting it changes again.
                Qt.callLater(endStroll);
                waitingOn = wanted;
                resting.restart();
            }
        }
    }

    // While it waits for the keys to rest on another column, it stays on
    // the window it sat on wherever niri moves it, as niri scrolls the
    // strip to a column off the screen's edge, never left on the air where
    // that window was or on one it never walked to (Joel, 2026-11-14
    // 17:24: "focusing to a window off the left or right edge"). Its place
    // is where niri says the window will come to rest, off the screen too,
    // and it's drawn on niri's spring from where it was (lag), so it goes
    // with the window as niri draws it. Once the keys rest, it comes back
    // in at that edge (MarginCatStacked.js's walk).
    function carried(saw: var): void {
        const by = MarginCat.carries(saw, tilesOf(space?.id), sat);
        if (by === null || place === null)
            return;
        rideTo({ x: place.x + by.x, y: place.y + by.y });
    }
    // Whether niri's scroll of its workspace still runs: told less than
    // its spring's length ago, since niri never says one ended, so an old
    // scroll's word, still on its windows, isn't taken for a new one
    // (MarginCatTiles.js's told). Heard again with each layout niri sends.
    function scrolling(): bool {
        const now = Date.now();
        scrollHeard = Tiles.hears(scrollHeard, Tiles.told(Niri.windows, space?.id, "ViewMove"),
            now);
        return Tiles.runs(scrollHeard, now, slideMs);
    }
    property var scrollHeard: null
    readonly property var spring: Object.assign({}, Theme.motion.spring,
        { slowdown: Theme.motion.slowdown })
    readonly property int slideMs: MarginCat.slides(spring)
    // Its place set to `to` as niri's scroll sets off, drawn where it was
    // as niri set it off and closing on `to` on niri's spring
    // (MarginCatStacked.js's rides).
    function rideTo(to: var): void {
        const now = Date.now();
        // As far into the scroll as niri says, or a guess at how late it
        // hears of one where niri says nothing or tells of one ended.
        const behind = MarginCat.behindTold(frameMs, "slide", scrolling()
            ? Tiles.animating(Niri.windows, space?.id, "ViewMove") : null);
        carry = Stacked.rides(carry, place, to, { at: now - behind, spring });
        lag = MarginCat.lags(carry.from, now - carry.at, spring);
        place = to;
    }
    // Where it's drawn: its place, short of it on a ride by its lag.
    function drawnAt(): var {
        return place === null ? null : { x: place.x + lag.x, y: place.y + lag.y };
    }
    // A ride ended where it's drawn, so whatever it does next, a walk, a
    // move or a lift, sets off from there, never from its place a lag on.
    function unride(): void {
        if (carry !== null)
            place = drawnAt();
        carry = null;
        lag = { x: 0, y: 0 };
    }
    // The scroll carrying it, while niri's spring runs, and how far from
    // its place that has it drawn now.
    property var carry: null
    property var lag: ({ x: 0, y: 0 })
    FrameAnimation {
        running: window.carry !== null
        onTriggered: {
            const t = Date.now() - window.carry.at;
            // A clock stepped back ends it too, rather than holding it
            // drawn off its place until the clock comes round again.
            if (t >= window.carry.ends || t < 0) {
                window.carry = null;
                window.lag = { x: 0, y: 0 };
                // Its ride done, it says it's drawn at its place.
                window.say();
            } else {
                window.lag = MarginCat.lags(window.carry.from, t, window.carry.spring);
            }
        }
    }

    // Sets off from where it's drawn to where it'd sit: `hurried`,
    // its seat moved on the way, without turning, thinking or
    // crouching again, or, setting off, after a thought of 150 to
    // 450 ms, so no two walks begin alike; riding what niri moved of
    // the windows it `saw` (MarginCatFalls.js's rides).
    function walkFrom(hurried: bool, saw: var): void {
        walkTo(hurried, saw, there);
    }

    // The window it stands on: the one its walk goes to, walking, or
    // the one it sat on.
    function standsOn(): var {
        return going === null ? sat : goingTo;
    }

    // Whether the window it sat on closed under it a moment ago, and it
    // hasn't walked off since, so the next change of its column, or of its
    // walk's target, is that close's.
    function fallsYet(): bool {
        return closedOn !== null && Date.now() - closedAt < MarginCat.CLOSING;
    }

    // Its workspace emptied, it falls from where it is to the floor
    // below and sits there, crossing only the backdrop; with nowhere
    // to fall from, the shell just started, it's there at once, in the
    // middle of the floor. Said again as niri says the layout, a fall
    // under way or done goes on as it is. It rides nothing: the windows
    // it stood on are gone, not moved.
    function floors(): void {
        const floor = modelData.height - size;
        if (place === null) {
            place = { x: (modelData.width - size) / 2, y: floor };
            return;
        }
        if (moving !== null || (going !== null && Math.abs(going.to.y - floor) < 0.5)
            || (going === null && Math.abs(place.y - floor) < 0.5))
            return;
        resting.stop();
        endStroll();
        // Carried off the screen's side with its window (carried), it's
        // out of sight on the floor past that edge, and comes in head first
        // to sit just inside it (Stacked.comesIn), never on a seat where it
        // can't be seen.
        const off = place.x + size <= 0 ? -1 : place.x >= modelData.width ? 1 : 0;
        if (off !== 0) {
            halt();
            place = { x: off < 0 ? -size : modelData.width, y: floor };
        }
        walkTo(true, null, {
            x: off < 0 ? 0 : off > 0 ? modelData.width - size : place.x,
            y: floor
        });
    }

    function walkTo(hurried: bool, saw: var, to: var): void {
        roams = null;
        landedOn = null;
        if (place === null || to === null)
            return;
        const tiles = tilesOf(space?.id);
        const heard = MarginCat.closeHeard({
            on: closedOn,
            at: closedAt,
            saw: closedSaw,
            gone: standsOn() !== null && Niri.windows[standsOn()] === undefined,
            stands: standsOn(),
            seen: saw
        }, Date.now());
        // A walk set off mid-stride strides on from the one drawn, as
        // far into it as it was, easing in again unless it was at its
        // pace; from a stride held on the spot, once it's seen.
        const setOff = Paths.setOffFrom(stepped);
        const from = drawnAt();
        const plan = Stacked.walkAlong(from, { x: to.x, y: to.y }, Object.assign({
            cell,
            ratio,
            wide: size,
            tall: size,
            air: cell,
            across: modelData.width,
            pause: 150 + Math.random() * 300,
            slowdown: Theme.motion.slowdown,
            hurried,
            height: modelData.height,
            cols: Paths.columns(tiles, 0),
            before: heard.before ? Paths.columns(heard.before, 0) : null,
            floats: Floats.of(tiles, 0),
            floated: heard.before ? Floats.of(heard.before, 0) : [],
            spring: Theme.motion.spring,
            // The window it stands on gone, until it sits again, or
            // closed a moment ago (MarginCat.closeHeard).
            closed: heard.closed,
            // Gone too where the window over it has grown over it first.
            on: heard.on,
            ahead: MarginCat.behind(frameMs, "resize"),
            // As far into a window's growth over it as niri says it is.
            grows: MarginCat.behindTold(frameMs, "resize",
                Tiles.animating(Niri.windows, space?.id, "Resize")),
            frame: frameMs,
            marks: marksLit
        }, setOff));
        goingTo = wanted;
        under = column;
        if (!fallsYet())
            closedOn = null;
        if (plan === null) {
            halt();
            sat = wanted;
            place = to;
            sits();
            return;
        }
        walks(plan);
    }

    // A walk's `plan` played from where it is, from its first step.
    // A walk's frames are counted from begunAt, read once `going` has
    // set following, so no frame it counts begins unasked.
    function walks(plan: var): void {
        const fresh = going === null;
        if (fresh) {
            sayWalked();
            shown = [];
            costs = [];
            unseen = 0;
            unheld();
            steps = 0;
        }
        byHand = false;
        // Set off before a scroll's spring settled, it walks from where
        // that had it drawn, as its plan does.
        unride();
        going = plan;
        if (fresh)
            begunAt = window.FramePresentation.framesBegun;
        goes(walker);
    }

    // A move to the workspace `way` below (1) or above (-1): it rides the one it leaves off
    // the top going down, or off the bottom going up, and its way to its new seat comes
    // once niri says the new workspace's column (steer).
    function moveTo(way: int): void {
        landedOn = null;
        halt();
        // A close is owned only by the walks it set going.
        closedOn = null;
        resting.stop();
        leaving.stop();
        movingTo = null;
        moving = plan(way, { x: place.x, y: place.y }, null);
        sayWalked();
        shown = [];
        costs = [];
        unseen = 0;
        unheld();
        steps = 0;
        begunAt = window.FramePresentation.framesBegun;
        byHand = false;
        goes(mover);
    }

    // A move's plan from `from` to its seat `to` over the new workspace's
    // windows, its floating ones too, and going down the old one's it
    // walks off, or only the slide where `to` is null.
    function plan(way: int, from: var, to: var): var {
        const tiles = tilesOf(space?.id);
        return Stacked.moves(from, to, {
            way,
            wide: size,
            tall: size,
            air: cell,
            cell,
            ratio,
            facing,
            across: modelData.width,
            height: modelData.height,
            slowdown: Theme.motion.slowdown,
            spring: Theme.motion.spring,
            ahead: MarginCat.behind(frameMs, "slide"),
            frame: frameMs,
            now: tiles,
            was: tilesOf(spaceLeft),
            floats: Floats.of(tiles, 0),
            marks: marksLit
        });
    }

    // Where a move goes as niri says the new workspace's columns: as it
    // holds, or past its hold with no seat yet, its way found again each
    // time niri says any, since none of the hold depends on them, and
    // a seat said late sets off from the hold then (Paths.resumed);
    // once it's on its way, a seat moved or the keys gone to another
    // window cut it short, and it walks on from where it's got to
    // (cut()).
    function steer(): void {
        if (there === null)
            return;
        const t = mover.elapsedTime * 1000;
        const holding = Paths.replans(moving, t, steps > 0);
        const same = Math.round(there.x * ratio) === Math.round(moving.to.x * ratio)
            && Math.round(there.y * ratio) === Math.round(moving.to.y * ratio);
        if (holding || movingTo === null) {
            movingTo = wanted;
            moving = Paths.resumed(plan(moving.way, moving.from, { x: there.x, y: there.y }), t);
        } else if (wanted !== movingTo || !same) {
            cut();
        }
    }

    // A move cut short past its hold, the keys gone to another window
    // or its seat moved: on from wherever it's got to, as a walk, so
    // it's never put anywhere it didn't get to (WCJQSG 2bd7c7c6).
    function cut(): void {
        tell("move");
        mover.stop();
        moving = null;
        movingTo = null;
        // No halt(): its pose and facing carry into the walk, never a
        // frame of it sitting between.
        walkFrom(true, null);
    }

    // Done moving, or cut short: it sits where the keys' column has
    // it, at once, or nowhere, as it would have with no move.
    function arrive(): void {
        tell("move");
        mover.stop();
        moving = null;
        movingTo = null;
        turn = 0;
        halt();
        sat = wanted;
        // Its ride done over an empty workspace, it falls to the floor
        // from just over the screen where it rode the old one off the
        // top, or springs up onto it from just under the screen where it
        // rode the old one off the bottom.
        if (there === null && emptied && place !== null && place.y >= modelData.height) {
            place = Stacked.risesFrom(place, { height: modelData.height, tall: size });
            goingTo = wanted;
            walks(Stacked.risesOnto(place, {
                cell,
                wide: size,
                tall: size,
                air: cell,
                height: modelData.height,
                facing: facing,
                slowdown: Theme.motion.slowdown
            }));
        } else if (there === null && emptied && place !== null) {
            place = Stacked.dropsFrom(place, { tall: size });
            floors();
        }
        else
            place = there;
    }

    onSpaceChanged: {
        const was = spaceWas;
        spaceWas = space;
        if (was === null || space === null || was.id === space.id)
            return;
        const way = Math.sign(space.idx - was.idx);
        const kept = place === null ? "nowhere" : way === 0 ? "level"
            : window.scene.asleep ? "asleep" : Niri.overview ? "overview"
            : window.scene.shared ? "shared" : "";
        // Which of those kept it from moving, so a test that waits on a
        // move it never made says why (Y2YBTS, 2026-11-14, a0cd2ad5).
        if (Options.reports)
            Report.say(`moves cat ${named} ${JSON.stringify({
                from: was, to: space, way, kept, walking: going !== null,
                moving: moving !== null })}`);
        // A move begun on a move under way holds where it's got to,
        // rather than being put on its seat first.
        spaceLeft = was.id;
        if (kept === "")
            moveTo(way);
        else if (moving !== null)
            arrive();
    }

    // How its walk, move or play (`what`, its kind) reached the
    // screen, in the journal, so one on Joel's screen says whether it
    // kept to its rate: told once niri has answered for the frames
    // still on their way as it sat (Shown.heard).
    function tell(what: string): void {
        if (!window.FramePresentation.available)
            return;
        sayWalked();
        telling = what;
        tellsAt = window.FramePresentation.framesBegun;
        // A hold still open as it ends is left out as well as those past,
        // and counted with them.
        const open = Launched.up && heldAt >= 0 ? heldAt : null;
        tellsBegun = Shown.begun(tellsAt, begunAt, heldBegun, open);
        if (open !== null)
            resumedAt = resumedAt.concat([shown.length]);
        tellsStepped = steps;
        answering.restart();
    }
    // The walk waiting to be told, told with the frames heard of so
    // far: once niri has answered for them all, its wait has run out,
    // or another sets off and needs its counts.
    function sayWalked(): void {
        if (telling === "")
            return;
        answering.stop();
        const what = telling;
        telling = "";
        const told = Shown.walked(shown, costs, unseen,
            { begun: tellsBegun, stepped: tellsStepped, what, resumed: resumedAt });
        console.info(told.line);
        if (told.said.frames > 10)
            frameMs = told.said.median;
        if (Options.reports)
            Report.say(`${what === "walk" || what === "move" ? "walked" : "played"} cat `
                + `${named} ${JSON.stringify(told.said)}`);
    }
    // Counted among the screens that draw it, so a cat drawn on none
    // keeps still.
    readonly property bool placed: !!place
    onPlacedChanged: window.scene.placedOn += placed ? 1 : -1
    Component.onDestruction: {
        gone = true;
        if (placed)
            window.scene.placedOn -= 1;
        if (wantsAs !== "")
            Niri.wantTiles(wantsAs, false);
    }

    // Every frame of a walk, a move or a play is asked of as it begins,
    // while it isn't held under the launcher, but for what Joel's hand
    // set off. Asked after each answer and at each step, a frame begun
    // before either reached the render thread went unasked: on Joel's
    // screen 30 of 44 walks, moves and plays from 22:00 to 01:30 on
    // 2026-11-22 told 1 to 7 frames fewer than they drew, each waiting
    // out its answers (Shown.follows, its cases in Qt's engine).
    FramePresentation.following: Shown.follows(window.FramePresentation.available,
        going !== null || moving !== null, Launched.up, byHand)
    // Each frame of it as niri showed it or didn't. Once it's done, the
    // answers still to come are only heard: each frame on its way was
    // asked of as it began.
    FramePresentation.onPresented: (frame, at) => {
        restSeen(frame);
        if (going === null && moving === null && telling === "")
            return;
        const heard = Shown.heard(frame, begunAt, telling === "" ? null : tellsAt);
        if (heard.counts) {
            shown.push(at);
            const times = window.FramePresentation.timesOf(frame);
            if (times.rendered > 0 && times.synchronizing > 0)
                costs.push(times.rendered - times.synchronizing);
        }
        answered(heard);
    }
    FramePresentation.onDiscarded: frame => {
        restSeen(frame);
        if (going === null && moving === null && telling === "")
            return;
        const heard = Shown.heard(frame, begunAt, telling === "" ? null : tellsAt);
        if (heard.counts)
            unseen += 1;
        answered(heard);
    }
    // After a frame's answer, `heard` (Shown.heard): the walk told once
    // every frame of it is in.
    function answered(heard: var): void {
        if (heard.ends)
            sayWalked();
    }
    // The longest a walk done waits to hear of its last frames.
    Timer {
        id: answering
        interval: Shown.ANSWERS
        onTriggered: window.sayWalked()
    }

    function halt(): void {
        unride();
        walker.stop();
        going = null;
        // After `going`, whose handler fades the yarn as the play that
        // ends was set off.
        byHand = false;
        stepped = null;
        pose = "";
    }

    // A step of a walk or a move, `now` (Paths.frame), taken: its pose,
    // the way it faces and turns, and its place, each set only as it
    // changes.
    function took(now: var): void {
        if (now.facing !== facing)
            facing = now.facing;
        if ((now.pose ?? "") !== pose)
            pose = now.pose ?? "";
        if ((now.turn ?? 0) !== turn)
            turn = now.turn ?? 0;
        stepped = now;
        steps += 1;
        place = { x: now.x, y: now.y };
    }

    // A scene's walk done, handing the scene its next beat.
    function sits(): void {
        if (!handsBack)
            return;
        handsBack = false;
        Qt.callLater(window.scene.beat);
    }

    // A stroll ended by its column going or another's taking the
    // keys: its wander is forgotten, and the scene moves on rather
    // than waiting on a walk that's gone.
    function endStroll(): void {
        strolled = null;
        roams = null;
        sits();
    }

    // A stroll's walk, where the scene says, taken only by the screen
    // whose cat sits still over the column the keys are in: a few
    // cells along the margin, then back over it, each with a thought and a
    // crouch as any walk; or, as a key ends the scene (`to` ""),
    // straight back over its column, hurried.
    function stroll(to: string): void {
        if (to === "") {
            handsBack = false;
            if (strolled !== null) {
                roams = true;
                strolled = null;
            }
            return;
        }
        if (place === null || column === null || going !== null || resting.running
            || wanted !== sat || holding)
            return;
        if (to === "play") {
            play(window.scene.playing);
            return;
        }
        if (to === "column") {
            if (strolled === null)
                return;
            window.scene.strollTaken = true;
            handsBack = true;
            roams = false;
            strolled = null;
            return;
        }
        const spot = Plays.wander({
            seat: seat(column),
            column,
            across: modelData.width,
            marks: MarginMarks.drawn,
            cell: Theme.gap,
            wide: size,
            step: cell,
            cells: window.scene.wandering?.cells ?? 0,
            way: window.scene.wandering?.way ?? 1
        });
        if (spot === null)
            return;
        window.scene.strollTaken = true;
        handsBack = true;
        roams = false;
        strolled = spot;
    }

    // A play that moves it (MarginCatGames.js), `asked` by the scene,
    // its kind and how long it thinks, from where it sits: run on the
    // walker as a walk is, its seat for the walk back where it lands.
    function play(asked: var): void {
        if (asked === null || holding || carry !== null)
            return;
        const playing = Games.play(asked.kind, playsFrom(asked.pause));
        if (playing === null)
            return;
        window.scene.strollTaken = true;
        handsBack = true;
        plays(playing, false);
    }

    // Its yarn by Joel's click on it (VKCF3Z 73c59942; Joel, 2026-11-12:
    // "i want to be able to also use my mouse to engage with my
    // animals."), rolled in where it sits still and awake with none out.
    // No scene's, so no scene waits on it.
    function clicked(): void {
        if (!still || yarnAt !== null || window.scene.asleep || place === null || column === null
            || resting.running || wanted !== sat)
            return;
        plays(Games.fetch(playsFrom(0)), true);
    }

    // A click on its yarn lying by it, on the yarn's `side` (-1 its left,
    // 1 its right): the yarn rolls away from it and the cat goes after it,
    // then sits. Its yarn kept again only once it's done, so a key cutting
    // the chase short fades it where it rolled.
    function batted(side: int): void {
        if (!yarnLies || !still || window.scene.asleep || place === null || column === null
            || resting.running || wanted !== sat)
            return;
        const playing = Games.chase(playsFrom(0), yarnAt, side, Math.random());
        if (playing === null)
            return;
        yarnKept = false;
        plays(playing, true);
    }

    // Picked up by Joel's hand (VKCF3Z 9ead6710; Joel, 2026-11-12: "i want
    // to be able to also use my mouse to engage with my animals."): held by
    // the scruff, still, drawn again only as the pointer moves, with no
    // timer, wherever it was going or playing, a scene's walk handed back;
    // its yarn fades. A press on it dozing or asleep wakes it as any press
    // does, being input. Let go of, it drops (drops()).
    property bool holding: false
    onHoldingChanged: say()
    // What it was last let go onto (Games.drop's `landsOn`, { floats: false }
    // for the floor), which says whether it's over the windows until it
    // next walks, moves or plays; null otherwise.
    property var landedOn: null
    onLandedOnChanged: say()
    function lift(): void {
        if (place === null || moving !== null)
            return;
        halt();
        closedOn = null;
        resting.stop();
        leaving.stop();
        endStroll();
        unpetted();
        landedOn = null;
        holding = true;
        byHand = true;
        // A play's yarn under the card was held fading as halt() ended
        // it, before his hand was on it, so it fades on now he has it.
        if (yarnAt !== null && yarnFades.paused)
            yarnFades.resume();
        else if (yarnAt !== null && !yarnFades.running)
            goes(yarnFades);
    }

    // Held with Joel's hand at `hand`, in the screen's logical pixels.
    function hold(hand: point): void {
        if (!holding || present === null)
            return;
        place = MarginCat.held(hand, {
            cell,
            wide: size,
            tall: size,
            across: present.width,
            height: present.height
        });
    }

    // Let go of, it drops from where his hand had it, then sits there till
    // the keys walk it on.
    function letGo(): void {
        if (!holding)
            return;
        drops();
        holding = false;
    }

    // A drop from where it is (Games.drop) onto what's under it on this
    // screen's workspace, the floating windows too; it stays where it lands,
    // its seat for any change of the windows until the keys move.
    function drops(): void {
        if (place === null || present === null)
            return;
        const tiles = tilesOf(space?.id);
        const playing = Games.drop(Object.assign(playsFrom(0), { air: cell }),
            Paths.columns(tiles, 0), Floats.of(tiles, 0));
        plays(playing, true);
        strolled = playing.to;
        landedOn = playing.landsOn ?? { floats: false };
    }

    // Where it plays from, as a play takes it (MarginCatGames.js), thinking
    // `pause` ms.
    function playsFrom(pause: real): var {
        return {
            seat: { x: place.x, y: place.y },
            column,
            across: modelData.width,
            height: modelData.height,
            marks: MarginMarks.drawn,
            gap: Theme.gap,
            cell,
            wide: size,
            tall: size,
            pause,
            slowdown: Theme.motion.slowdown
        };
    }

    // A play, `playing`, run on the walker as a walk is, set off by Joel's
    // `hand` or not.
    function plays(playing: var, hand: bool): void {
        if (playing === null)
            return;
        closedOn = null;
        roams = null;
        landedOn = null;
        sayWalked();
        shown = [];
        costs = [];
        unseen = 0;
        unheld();
        steps = 0;
        goingTo = wanted;
        under = column;
        byHand = hand;
        going = playing;
        begunAt = window.FramePresentation.framesBegun;
        goes(walker);
        // Set once it's going, so the seat moving under it is its
        // own walk's end, and sets no walk off.
        if (playing.to.x !== place.x || playing.to.y !== place.y)
            strolled = playing.to;
    }

    // Its yarn where a play has it, or null; and, a key cutting the
    // play short, how much of it is left as it fades where it lay,
    // rather than vanishing. A click's play leaves it lying (`yarnKept`),
    // drawn once and still, with no timer, till a click on it sends it
    // on, or any walk, other play or doze fades it.
    property var yarnAt: null
    property real yarnLeft: 1
    property bool yarnKept: false
    readonly property bool yarnLies: yarnKept && yarnAt !== null && !yarnFades.running
    onYarnLiesChanged: say()
    onGoingChanged: {
        if (yarnAt === null || yarnFades.running)
            return;
        if (going === null ? !yarnKept : going.kind === undefined
            || (yarnKept && going.kind !== "chase"))
            goes(yarnFades);
    }
    NumberAnimation {
        id: yarnFades
        target: window
        property: "yarnLeft"
        from: 1
        to: 0
        duration: Theme.motion.short
        onFinished: {
            window.yarnAt = null;
            window.yarnKept = false;
            window.yarnLeft = 1;
        }
    }

    Connections {
        target: window.scene

        function onStrollToChanged() {
            window.stroll(window.scene.strollTo);
        }
        function onSettledChanged() {
            window.scrolling();
        }
    }

    onThereChanged: follow()

    // Nowhere, once it's held a moment: niri can say a column moving
    // to the keys' as nowhere between two of its events, which gone at
    // once had it vanish and land on the new one with no walk (a
    // builder, 2026-11-12, 2 of 4 runs), and a column truly gone, to
    // fullscreen or another workspace, takes it a moment later.
    Timer {
        id: leaving
        interval: MarginCat.LEAVES
        onTriggered: {
            if (window.there !== null || window.emptied)
                return;
            window.halt();
            window.closedOn = null;
            window.endStroll();
            resting.stop();
            window.sat = window.wanted;
            window.place = null;
        }
    }

    // Under the launcher's card it holds where it got to and draws
    // nothing: a walk or a move is paused, never skipped ahead, and goes
    // on from there as the card goes, since a paused clock keeps its
    // elapsedTime, and niri isn't asked of its frames meanwhile
    // (following, above). A play off its seat ends as a key ends it, and
    // its walk back is held the same way (MarginCat.qml goes still).
    // Joel's keys are in the launcher then, and each of the cat's frames
    // is one the shell draws beside theirs: on his laptop it drew 10 to
    // 19 minutes an hour while he worked (2026-11-19), one walk running
    // under every round of a lab's keys.
    readonly property bool stilled: Launched.up
    onStilledChanged: {
        for (const clock of [walker, mover, yarnFades]) {
            if (stilled && clock.running && !byHand)
                clock.pause();
            else if (!stilled)
                clock.resume();
        }
        if (going === null && moving === null)
            return;
        if (stilled) {
            if (!byHand)
                heldAt = window.FramePresentation.framesBegun;
            return;
        }
        // None open, as for what his hand set off, or a walk set off by
        // another handler of this change once the card had gone.
        if (heldAt < 0)
            return;
        heldBegun += window.FramePresentation.framesBegun - heldAt;
        heldAt = -1;
        resumedAt = resumedAt.concat([shown.length]);
    }

    // Sets `clock`, the walker, the mover or the yarn's fading, going
    // from its start; held at once while the launcher's card is up, its
    // frames unasked (following, above), but for what Joel's hand set
    // off. It reads the card itself rather than
    // `stilled`, which another handler of the same change may run before.
    function goes(clock: var): void {
        clock.restart();
        if (Launched.up && !byHand) {
            clock.pause();
            if (clock !== yarnFades && heldAt < 0)
                heldAt = window.FramePresentation.framesBegun;
        }
    }

    // Until the keys rest on the column, so a tap along them never
    // sets it walking.
    Timer {
        id: resting
        interval: MarginCat.RESTS
        onTriggered: window.walkFrom(false, null)
    }

    // A step each time the screen draws, at whatever rate it draws,
    // and none once it sits.
    FrameAnimation {
        id: walker
        onTriggered: {
            const t = walker.elapsedTime * 1000;
            const kind = window.going.kind;
            const now = kind === undefined ? Paths.frame(window.going, t)
                : Games.frame(window.going, t);
            if (kind !== undefined && !yarnFades.running)
                window.yarnAt = now.yarn;
            if (now.done) {
                // A click's play leaves its yarn lying where it ends.
                if (window.yarnAt !== null && !yarnFades.running)
                    window.yarnKept = true;
                window.tell(kind ?? "walk");
                window.halt();
                // Off the window that closed under it, the close is done.
                window.closedOn = null;
                window.sat = window.goingTo;
                // Where its walk took it, which is its seat unless the keys
                // went elsewhere on the way, whose seat it walks to once
                // they rest there.
                window.place = { x: now.x, y: now.y };
                window.sits();
                return;
            }
            window.took(now);
        }
    }

    // A step of a move each time the screen draws, as the walker's.
    FrameAnimation {
        id: mover
        onTriggered: {
            const t = mover.elapsedTime * 1000;
            const now = Paths.frame(window.moving, t);
            // A hold with no seat yet holds on a moment for one niri
            // says late (Paths.holdsOn), where it is, rather than going
            // and coming back with no walk; with none by then, or on an
            // empty workspace, it arrives, where it falls to the floor.
            const waits = window.movingTo === null && !window.emptied
                && Paths.holdsOn(window.moving, t);
            if (!waits && (now.done
                || (now.beat !== window.moving.beats[0].name && window.movingTo === null))) {
                window.arrive();
                return;
            }
            if (!now.done)
                window.took(now);
        }
    }

    // A window's tile where it's on this screen's shown workspace
    // (MarginCatTiles.js); null while the screen is null or gone, as
    // Variants leaves it a moment as screens come and go (Joel's shell,
    // 2026-11-12 20:11, "Cannot read property 'name' of null").
    function tile(id: var): var {
        return present === null ? null
            : Tiles.tile(Niri.windows, Niri.workspaces, present.name, id);
    }

    // The tiles of a workspace's windows, as Paths.columns takes them.
    function tilesOf(workspaceId: var): var {
        return Tiles.of(Niri.windows, workspaceId);
    }

    // Nowhere over no column, or on a screen going away, whose width
    // is null (vm-screens-keep-the-launcher, "Cannot read property
    // 'width' of null", 4 runs of 4 on 2026-11-12).
    function seat(over: var): var {
        if (!over || present === null)
            return null;
        return MarginCat.sits({
            column: over,
            // The screen's, since the window's own is 0 until it's shown.
            across: present.width,
            marks: MarginMarks.drawn,
            cell: Theme.gap,
            wide: size,
            tall: size,
            air: cell
        });
    }

    Counted { name: `cat ${window.named}` }

    screen: modelData
    // A place is undefined, not null, while a binding works it out
    // again, so both are nowhere (Joel's shell, 2026-11-12: "Cannot
    // read property 'x' of undefined").
    visible: !!place && window.scene.drawing.length > 0
    anchors.top: true
    anchors.left: true
    anchors.right: true
    // The screen's height, as it gets about the windows below, even at
    // rest: grown only as a move set off, its first frames waited on
    // niri's new size, so its ride began out of sight and showed only as
    // it ended (the GPU rig, 2026-11-13: 141, 331, then 801 px down).
    // Only its own cells take the pointer, and Qt draws it only as it moves.
    // Its screen is null or gone for a moment as the screen goes, and the
    // binding reads it again then (vm-screens-keep-the-launcher, 2026-11-15
    // and 2026-11-17), so it reads the screen only while it's listed.
    implicitHeight: present?.height ?? 0
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    // Under the margin's marks, the launcher and notices, on Overlay, so
    // a mark's ink lies over its paws as it perches there and it never
    // hides a mark or a notice (VKCF3Z, 3d799798); and under the windows,
    // but on a floating one's top over them (MarginCat.above). It leaves
    // that seat on a walk to another window or a move between workspaces.
    // Its seat is read as niri has it now, not as it was when it set off:
    // a window opened tiled and made floating as it walked there is a
    // floating one by the time it sits (a nested niri, 2026-11-15). Not
    // named `leaving`, the timer's id, which a bare name finds first. Held, and as it falls
    // from his hand, it's over them all, then as what it landed on says (MarginCat.inFront);
    // its first frame let go, before its first step, still falls.
    readonly property bool departing: moving !== null || (going !== null && goingTo !== sat)
    readonly property var seatedOn: {
        void window.scene.settled;
        void Niri.revision;
        return window.scene.shared || Niri.overview ? null : tile(sat);
    }
    // Over them too on each leg of a walk aboard a floating window, onto
    // it, along it or off it, from that leg's first drawing to its end.
    readonly property bool above: MarginCat.inFront({
        holds: holding,
        falls: going?.kind === "drop" && (stepped === null || stepped.beat === "falls"),
        landed: landedOn,
        seat: seatedOn,
        leaving: departing,
        aboard: stepped?.aboard
    })
    onAboveChanged: say()
    WlrLayershell.layer: above ? WlrLayer.Top : WlrLayer.Bottom
    WlrLayershell.namespace: "cat"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    // Only its own cells take the pointer, to be petted, so the gap
    // around it stays the windows' and a click beside it reaches them.
    mask: Region {
        item: cat

        Region {
            item: yarnHit
            intersection: Intersection.Combine
        }
    }

    // The pointer over it at `at`, in the screen's pixels, `across` its
    // cells: one that moved onto it as it sat still starts a pet.
    function felt(at: point, across: real): void {
        const rested = still && stillSince >= 0 ? KeyTimes.now() - stillSince : -1;
        const moved = MarginCat.strokes(pointer, at, rested);
        // As the pointer comes onto it and as it strokes it, it says how
        // long it had rested and where the pointer was, so a test that
        // finds it leaning on a pointer it came to under can say why.
        if (Options.reports && (pointer === null || moved))
            Report.say(`feels cat ${named} ${JSON.stringify({
                was: pointer,
                at: { x: at.x, y: at.y },
                rested: rested,
                strokes: moved,
                reach: window.implicitHeight
            })}`);
        pointer = { x: at.x, y: at.y };
        window.across = across;
        if (moved && leaning === 0 && !petting.running) {
            leaningTo = MarginCat.leansTo(across, size);
            petting.restart();
        } else if (leaning !== 0) {
            petting.stop();
        }
    }

    // The pointer gone from it: a pet not yet come never comes, though
    // one that only crossed it may have it swat after it, and a lean is
    // held a moment before it sits again. A swat under way plays out.
    function unfelt(): void {
        pointer = null;
        if (swatting !== 0)
            return;
        const crossed = leaning === 0 && petting.running && still;
        if (leaning !== 0) {
            petting.restart();
        } else if (MarginCat.swats(crossed, Date.now() - swattedAt, window.scene.asleep,
            Math.random())) {
            swatting = MarginCat.leansTo(across, size);
            swattedAt = Date.now();
            petting.restart();
        } else {
            petting.stop();
        }
    }

    // No pet coming, no lean and no swat, as it sets off or dozes.
    function unpetted(): void {
        petting.stop();
        leaning = 0;
        swatting = 0;
    }

    // Its one timer for a pet and a swat: run only while a pointer comes
    // onto it or leaves it, and for a swat's paw, never at rest.
    Timer {
        id: petting
        interval: window.swatting !== 0 ? MarginCat.SWATS
            : window.leaning === 0 ? MarginCat.PETS : MarginCat.PETTED
        onTriggered: {
            if (window.swatting !== 0)
                window.swatting = 0;
            else if (window.leaning !== 0)
                window.leaning = 0;
            else if (hand.hovered && window.still && !window.scene.asleep)
                window.leaning = window.leaningTo;
        }
    }

    // Where it sits, or that it's nowhere, for a test to read.
    // It says the column it saw too, so a test that finds it nowhere
    // says why, and its cells in device pixels and its hue, so one
    // can count them on the screen. On a walk it says each frame's
    // step, its time since it set off, beat and pose, so a test can
    // see a frame dropped or a stride out of step.
    function say(): void {
        if (!Options.reports)
            return;
        const at = !place ? "nowhere" : JSON.stringify({
            x: place.x,
            y: place.y,
            // Where it's drawn, behind its place as a scroll carries it.
            drawn: { x: place.x + lag.x, y: place.y + lag.y },
            size: window.size,
            air: window.cell,
            cell: Math.round(window.cell * window.ratio),
            cells: window.drawn.length,
            lit: window.drawn.join("").split("#").length - 1,
            hue: `${Theme.voice}`,
            pose: window.scene.posed,
            strolling: handsBack || strolled !== null,
            window: sat,
            // Whether it asks to be over the windows, which niri takes
            // only once its surface is next drawn, and what that's from.
            above: window.above,
            seat: seatedOn,
            departing: departing,
            asleep: window.scene.asleep,
            stretching: window.scene.stretching,
            leaning: window.leaning,
            swatting: window.swatting,
            held: window.holding,
            // Whether it's held still under the launcher's card, which
            // what Joel's hand set off never is.
            stilled: window.stilled && !window.byHand,
            landed: window.landedOn,
            yarn: yarnLies ? { x: yarnAt.x, y: yarnAt.y } : null,
            walk: going === null || stepped === null ? null : {
                t: Math.round(walker.elapsedTime * 10000) / 10,
                beat: stepped.beat,
                pose: stepped.pose,
                facing: stepped.facing,
                stride: going.stride,
                to: going.to.x,
                beats: going.beats.map(beat => beat.name),
                play: going.kind ?? null,
                yarn: yarnAt
            },
            move: moving === null || stepped === null ? null : {
                t: Math.round(mover.elapsedTime * 10000) / 10,
                beat: stepped.beat,
                pose: stepped.pose,
                turn: stepped.turn,
                facing: stepped.facing,
                way: moving.way,
                beats: moving.beats.map(beat => beat.name),
                // How far niri's slide still has its ground off
                // where its way lies, so a check can tell a ride
                // from a jump.
                carried: Math.round((stepped.carried ?? 0) * 10) / 10
            },
            // The layer's height, the screen's while it has one and 0
            // as it goes, so a check can tell a place below it from one
            // on it.
            reach: window.implicitHeight
        });
        Report.say(`sits cat ${named} ${at} over ${JSON.stringify(column)}`);
    }
    onPlaceChanged: say()
    onColumnChanged: say()
    // A new scale can leave its logical place and size as they were
    // while its cells take more device pixels, so it says so too.
    onRatioChanged: say()
    readonly property bool asleep: window.scene.asleep
    onAsleepChanged: {
        if (asleep)
            unpetted();
        if (asleep && yarnLies)
            goes(yarnFades);
        say();
    }
    readonly property bool stretching: window.scene.stretching
    onStretchingChanged: say()
    // A scene's pose changes nothing of its place, so it says so too.
    readonly property string posed: window.scene.posed
    onPosedChanged: say()
    // One still as it's made rests from there, since still's first value
    // calls no onStillChanged. On a nested niri's start it comes to be
    // still only after it's made, so a pet reaches it either way there
    // (2026-11-24); this is for a screen where it's still from the first.
    Component.onCompleted: {
        follow();
        if (still)
            rests();
        say();
    }

    Item {
        id: cat

        // On the screen's own pixels, so every cell's edge is sharp,
        // a hop's height too.
        x: window.place ? Math.round((window.place.x + window.lag.x) * window.ratio)
            / window.ratio : 0
        // Held inside the margin, but on a move down, carried up off it
        // and climbing back.
        y: !window.place ? 0
            : window.moving !== null
            ? Math.round(window.place.y * window.ratio) / window.ratio
            : Math.max(0, Math.round(Math.min(window.place.y + window.lag.y,
                window.height - window.size) * window.ratio) / window.ratio)
        width: window.size
        height: window.size

        // The pointer over its cells, which never changes its shape.
        HoverHandler {
            id: hand

            onHoveredChanged: hovered ? window.felt(point.scenePosition, point.position.x)
                : window.unfelt()
            onPointChanged: {
                if (hovered)
                    window.felt(point.scenePosition, point.position.x);
            }
        }

        // A click on it rolls its yarn in.
        TapHandler {
            onTapped: window.clicked()
        }

        // A press on it that moves 4 px picks it up, and letting go drops
        // it. Only its cells take the press; the press's own grab carries
        // the pointer's moves and its release wherever it goes after. Never
        // while the launcher or a notice is in front, which take the press
        // over it on Overlay.
        DragHandler {
            target: null
            dragThreshold: MarginCat.LIFTS
            enabled: window.moving === null && window.placed
            onActiveChanged: active ? window.lift() : window.letGo()
            onCentroidChanged: {
                if (active)
                    window.hold(centroid.scenePosition);
            }
        }

        Repeater {
            model: Pixels.runs(window.drawn)

            Rectangle {
                required property var modelData

                // A lean to its left starts a cell out (MarginCat.leansOut),
                // and a swat to its left three (MarginCat.swatsOut).
                x: (modelData.x - MarginCat.leansOut(window.leaning)
                    - MarginCat.swatsOut(window.swatting)) * window.cell
                y: modelData.y * window.cell
                width: modelData.across * window.cell
                height: window.cell
                antialiasing: false
                color: Theme.voice
            }
        }
    }

    // Its ball of yarn, where a play has it, in the cat's own cells and
    // hue, on the screen's own pixels as the cat is, and drawn only within
    // its room, so it rolls out from behind the room's edge and back
    // behind it.
    Item {
        x: window.yarnAt ? window.snapped(window.yarnAt.within.left) : 0
        width: window.yarnAt
            ? window.snapped(window.yarnAt.within.right) - window.snapped(window.yarnAt.within.left)
            : 0
        height: parent.height
        clip: true
        opacity: window.yarnLeft
        visible: window.yarnAt !== null

        Item {
            x: window.yarnAt ? window.snapped(window.yarnAt.x) - parent.x : 0
            y: window.yarnAt ? window.snapped(window.yarnAt.y) : 0
            width: window.size
            height: window.size

            Repeater {
                model: Pixels.runs(window.yarnDrawn)

                Rectangle {
                    required property var modelData

                    x: modelData.x * window.cell
                    y: modelData.y * window.cell
                    width: modelData.across * window.cell
                    height: window.cell
                    antialiasing: false
                    color: Theme.voice
                }
            }
        }
    }

    // Where the yarn lying by it takes a click, on its cells only while
    // it lies there, so they leave the input region as its fade ends.
    Item {
        id: yarnHit

        x: window.yarnAt ? window.snapped(window.yarnAt.x) : 0
        y: window.yarnAt ? window.snapped(window.yarnAt.y) : 0
        width: window.yarnKept && window.yarnAt !== null ? window.size : 0
        height: width

        TapHandler {
            onTapped: eventPoint => window.batted(eventPoint.position.x < window.size / 2 ? -1 : 1)
        }
    }

    // The yarn's drawing, built again only as its pose or facing changes,
    // never each frame it rolls: a new drawing each frame had its cells
    // made again each frame, and a play cost the shell 4 to 7% of a core
    // (the GPU rig, 2026-11-14).
    readonly property string yarnLooks: MarginCat.yarnLooks(yarnAt)
    readonly property var yarnDrawn: MarginCat.yarnDrawn(yarnLooks, Theme.pixels)

    // Where `at` logical pixels falls on one of the screen's own.
    function snapped(at: real): real {
        return Math.round(at * ratio) / ratio;
    }
}
