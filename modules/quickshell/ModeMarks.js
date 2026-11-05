// The launcher's mode in three capitals, as the foot's start names it
// (Joel, 2026-11-05: "the mode should be 3 letters, in caps, and in a place
// where they are more obvious and not lost with other things"), worked out
// apart from the foot so tests/query-keys.qml can hold it in Qt's own
// engine, as held notices' own is (NoticeKeys.js).
.pragma library
.import "QueryKeys.js" as QueryKeys

const marks = { normal: "NOR", find: "FND", command: "CMD", insert: "INS", visual: "VIS" };

// The find line, insert, visual on the rows or the drawer's page, the
// command line whatever mode : came from, and normal for the rest, the
// drawer's page among them.
function mark(mode, commanding) {
    const named = commanding ? "command"
        : mode === "visual" || QueryKeys.chosen(mode) ? "visual" : mode;
    return marks[named] ?? marks.normal;
}
