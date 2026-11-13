// The line Joel says something on from anywhere, on Super+S (QCXZEB's slice B, e23364f8;
// VKCF3Z, e8b4139e): the launcher's head alone, a step inside the top of the focused screen,
// over his windows and taking no room from them, so the window he's talking about stays in view
// (Joel, 2026-11-12 16:00: talking to the void "ends up taking over my screen"). It opens in
// insert, who will hear it at its left as the launcher's Talk names its session: the focused
// window's session, else the one standing at Agents' top, the void (modules/agents/src/tell.rs).
// Escape gives normal, as the launcher's line does, and never puts it away; the chord again
// does, the words kept, as a switch keeps them (Drafts.qml). Return says them as a talk, and
// Told's quiet card says they went and then what came back.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Wayland
import QtQuick
import KeyTimes

Scope {
    id: root

    // The launcher, whose head this is, whose drafts it keeps and whose Agents it says through.
    required property var launcher
    // Whether it's up, and on which screen: the one focused as the chord came.
    property bool up: false
    property var screen: null
    // Who hears it, `{ to, name }`, once Agents has said; null while it's asking, and while
    // nobody would.
    property var hears: null
    property bool asked: false
    // Return pressed before Agents said who hears it, so it's said once it has.
    property bool waiting: false
    // What's typed on the line is kept under its own name until it's sent.
    readonly property string key: "line"
    readonly property var words: Words.of("agents:line")

    function agents() {
        return launcher.collections.find(one => one.settings.name === "agents") ?? null;
    }

    // `chord` is when launcher-say ran for it, on the wall clock in ms, or -1 where it didn't
    // say, as qs ipc doesn't.
    function toggle(chord) {
        if (up) {
            putAway();
            return;
        }
        // Read once, as the line comes up, in a function rather than a binding, so there's
        // nothing for a notify to update.
        const after = surface.FramePresentation.framesBegun; // qmllint disable stale-property-read
        timing = { heard: KeyTimes.now(), late: chord > 0 ? Math.max(0, Date.now() - chord) : -1,
            after };
        bringUp();
        surface.FramePresentation.ask();
    }

    // From its chord to niri presenting its first frame up, held to Joel's 50 ms (VKCF3Z,
    // e8b4139e), said once each time it comes up; null once said.
    property var timing: null
    function presented(frame, at) {
        if (timing === null)
            return;
        if (frame <= timing.after) {
            surface.FramePresentation.ask();
            return;
        }
        const shown = Math.round(at - timing.heard);
        const late = timing.late;
        console.info(late < 0 ? `talk line: up ${shown} ms after it was asked`
            : `talk line: up ${shown + late} ms after its chord, heard ${late} ms in`);
        timing = null;
    }

    function bringUp() {
        screen = launcher.focusedScreen;
        hears = null;
        asked = false;
        waiting = false;
        const kept = launcher.drafts.kept(key);
        prompt.text = kept.text;
        up = true;
        prompt.take();
        prompt.typing();
        if (kept.text !== "")
            prompt.caretTo(kept.caret);
        const from = agents();
        if (!from) {
            asked = true;
            return;
        }
        from.hears(Niri.focused, heard => {
            if (!root.up)
                return;
            root.hears = heard;
            root.asked = true;
            if (root.waiting)
                root.send();
        });
    }

    function putAway() {
        launcher.drafts.flush();
        up = false;
    }

    function send() {
        const text = prompt.text.trim();
        if (text === "")
            return;
        if (!asked) {
            waiting = true;
            return;
        }
        waiting = false;
        // Nobody to hear it keeps the words on the line, saying so in its place.
        if (hears === null)
            return;
        launcher.drafts.handed(key, hears.to, text);
        agents()?.say(hears, text);
        prompt.clear();
        up = false;
    }

    PanelWindow {
        id: surface

        Counted { name: "talk-line" }
        visible: true
        screen: root.screen ?? root.launcher.focusedScreen
        anchors.top: true
        anchors.left: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: root.up ? Theme.space.rung.screen - card.reach // qmllint disable unqualified
            : 1 - implicitHeight
        margins.left: !root.up || !surface.screen ? 1 - implicitWidth // qmllint disable unqualified
            : Math.round((surface.screen.width - root.launcher.width) / 2) - card.reach
        implicitWidth: root.launcher.width + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: root.up ? card.area : nowhere
        BackgroundEffect.blurRegion: root.up ? card.area : nowhere
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: root.up ? Niri.asksForKeys : WlrKeyboardFocus.None
        WlrLayershell.namespace: "talk-line"
        FramePresentation.onPresented: (frame, at) => root.presented(frame, at)
        FramePresentation.onDiscarded: frame => {
            if (root.timing !== null)
                surface.FramePresentation.ask();
        }

        Region {
            id: nowhere
        }

        Card {
            id: card
            x: reach
            y: reach
            width: root.launcher.width
            bands: true
            keys: root.up
            // Clear rather than hidden while put away, since a hidden field can't hold the
            // focus, and a letter typed as it comes would be lost.
            opacity: root.up ? 1 : 0

            Item {
                x: -card.inset
                width: card.contentWidth + 2 * card.inset
                height: root.launcher.headTall

                Prompt {
                    id: prompt
                    width: parent.width
                    height: parent.height
                    steady: true
                    modal: true
                    words: true
                    awake: root.up
                    taking: root.up
                    fade: Options.caret.fade
                    stopsAfter: Options.caret.stopsAfter
                    wait: Options.keyTiming.wait
                    glyph: ""
                    lead: Theme.glyph[root.words.glyph ?? ""] ?? ""
                    leadHue: Theme.voice
                    glyphWidth: root.launcher.textAreaAt
                    leadColumn: root.launcher.glyphColumn
                    // Who hears it, where the launcher's Talk names its session.
                    chips: [{ text: !root.asked ? root.words.said?.asking ?? ""
                        : root.hears?.name ?? root.words.said?.nobody ?? "", yields: false }]
                    hint: root.words.ask ?? ""
                    aside: ""
                    onAccepted: root.send()
                    onTextChanged: root.typed()
                    onCaretChanged: root.typed()
                }
            }
        }
    }

    function typed() {
        if (up)
            launcher.drafts.typed(key, prompt.text, prompt.caret);
    }

    // What the line shows, for the launcher's showing: whether it's up with the keys, what's
    // typed, its mode and caret, who it names, and how far down the screen its card stands.
    function shown() {
        return { up, keys: up && prompt.Window.active, typed: prompt.text, mode: prompt.mode,
            caret: prompt.caret, hears: prompt.chips[0]?.text ?? "",
            top: up ? surface.margins.top + card.reach : -1 };
    }
}
