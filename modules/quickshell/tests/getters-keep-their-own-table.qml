// Whether turning a field into a getter in one object leaves every other
// object's fields whole, in the Qt the shell is built with
// (modules/quickshell/qtdeclarative-accessor-keeps-its-own-table.patch).
// The `a-getter-turned-leaves-other-shapes-whole` check runs this as the
// shell. Two objects are made field by field, the second's shape grown from
// the first's, so they share one table of where each field lives; turning
// `verb` into a getter in the first, then in the second, put the second's
// setter over its `text` in Qt 6.12.0, which then read as the setter, and
// past the object's end in a longer shape, which crashed the shell on
// 2026-11-11. It exits 1, saying what was wrong, for any field read wrong.
import Quickshell
import QtQuick

ShellRoot {
    Component.onCompleted: {
        const make = extra => {
            const made = {};
            made.name = "name";
            made.verb = "verb";
            for (const field of extra)
                made[field] = field;
            return made;
        };
        const accessor = {
            enumerable: true,
            configurable: true,
            get: () => "got",
            set: value => {}
        };
        // The second's shape grown longer still, so a setter put at a slot
        // its first turn left behind would land past the shorter's end.
        const shapes = [[], ["text", "reads"], ["text", "reads", "across", "down", "about"]];
        const made = shapes.map(make);
        for (const one of made)
            Object.defineProperty(one, "verb", accessor);
        const wrong = [];
        made.forEach((one, at) => {
            if (one.verb !== "got" || one.name !== "name")
                wrong.push(`shape ${at}: verb ${one.verb}, name ${one.name}`);
            for (const field of shapes[at]) {
                if (one[field] !== field)
                    wrong.push(`shape ${at}: ${field} read as ${String(one[field]).slice(0, 40)}`);
            }
        });
        for (const one of wrong)
            console.warn(`a getter turned in another shape changed a field: ${one}`);
        // Said once the shell has loaded, as before it nothing hears it.
        Qt.callLater(() => Qt.exit(wrong.length === 0 ? 0 : 1));
    }
}
