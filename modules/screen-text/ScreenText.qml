// What the screen-text daemon has read gives Capture's Recent the
// screenshots whose words hold what's typed (`takenByText`), which Recent
// lists below those found by their names (launcher/Capture.qml). It lists
// no row of its own. The asking is Taken.qml's, on its one connection to
// the daemon, each text asked once; this says the launcher found more once
// an answer comes, so what's typed is searched again, and lets the
// connection go once the launcher closes. A machine without screen-text
// has no provider asking.
import qs
import QtQuick

Provider {
    id: provider

    // When the text now asked was first asked, so an answer that comes as
    // soon as the key is ranked as its own search.
    property real askedAt: 0
    // How soon that is: the daemon answers from what it keeps, in
    // milliseconds.
    readonly property int freshFor: 50

    // The screenshots whose words hold what's typed, best first, as
    // [{ path, line }], or none while they're asked.
    function takenByText(typed) {
        const records = Taken.search(typed);
        if (records === null)
            provider.askedAt = Date.now();
        return records ?? [];
    }

    // At home, where Capture gathers Recent's matches, what's typed waits
    // on the daemon's answer, asked now, as Taken asks it once a text.
    function awaits(letters) {
        if ((provider.launcher?.held.length ?? 0) !== 0 || Taken.search(letters) !== null)
            return false;
        provider.askedAt = Date.now();
        provider.awaited = Taken.searchedAs(letters);
        return true;
    }
    // The text a key waits on, which the launcher hasn't searched yet.
    property string awaited: ""

    // The launcher closing lets Taken's connection go, so none is held at
    // rest; the next search opens it again.
    Connections {
        target: provider.launcher

        function onOpenChanged(): void {
            if (!provider.launcher.open)
                Taken.rest();
        }
    }

    Connections {
        target: Taken

        // Only an answer to what's typed now: one to a text typed past
        // would search again for nothing new.
        function onSearched(words: string): void {
            if (Taken.searchedAs(provider.launcher?.typed ?? "") === words
                || provider.awaited === words)
                provider.foundMore(Date.now() - provider.askedAt < provider.freshFor);
        }
    }
}
