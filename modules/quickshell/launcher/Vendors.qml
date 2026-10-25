// Hardware (MAC) addresses, answered in the field with who made the device,
// from the IEEE's list of vendors kept on this machine, and what kind of
// address it is. A locally set address, as phones and laptops make up for
// each network to keep from being followed, has no vendor to name. The
// list is hwdata's, split at build time into a file for each first byte,
// named in `directory`, so a lookup reads one small file.
import qs
import Quickshell.Io
import QtQuick
import "Addresses.js" as Addresses

Provider {
    id: provider

    examples: ["3c:22:fb:12:34:56"]

    // Who made it, by the first three bytes, or "" when the list doesn't say.
    function vendor(oui) {
        if (!settings.directory)
            return "";
        list.path = `${settings.directory}/${oui.slice(0, 2)}`;
        const text = read(list);
        const at = text.indexOf(`\n${oui}\t`);
        if (at < 0)
            return "";
        const end = text.indexOf("\n", at + 1);
        return text.slice(at + oui.length + 2, end < 0 ? text.length : end);
    }

    function answer(typed) {
        // The command's word may lead it, as the commands list it.
        const at = Addresses.mac(typed.trim().replace(/^mac\s+(?=\S)/i, ""));
        if (!at)
            return null;
        const made = at.local ? "" : vendor(at.oui);
        // Who made it is its maker, as a display's or a keyboard's is.
        const headline = at.local ? "locally set, no maker" : made || "no maker listed";
        const card = Theme.glyph.card;
        // The maker is the headline, so the rows say only the rest, each
        // part named first.
        const kind = `${at.local ? "locally set" : "globally unique"}, `
            + `${at.multicast ? "multicast" : "unicast"}`;
        const rows = [];
        rows.push(copyRow({ name: Words.name("vendors:kind"), side: kind, copy: kind,
            glyph: card }));
        rows.push(copyRow({ name: Words.name("vendors:colons"), side: at.colon, copy: at.colon,
            glyph: card }));
        rows.push(copyRow({ name: Words.name("vendors:dotted"), side: at.dotted, copy: at.dotted,
            glyph: card }));
        return {
            headline: headline,
            copy: made || at.colon,
            rows: rows,
            note: at.local ? "" : "the IEEE\u2019s list, kept on this machine"
        };
    }

    FileView {
        id: list
        blockLoading: true
        printErrors: false
    }
}
