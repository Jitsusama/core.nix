// The shell's notices, for tests/read-there.qml: a history the case
// gives, read as Notices.qml reads one, by a new history whenever a notice
// unread is read, and each call that read one kept in `asked`.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var history: []
    property var asked: []

    function markRead(ids, how) {
        if (history.some(notice => ids.includes(notice.id) && !notice.read)) {
            asked = asked.concat([ids]);
            history = history.map(notice => !ids.includes(notice.id) ? notice
                : Object.assign({}, notice, { read: true, readHow: how ?? null }));
        }
    }
}
