// Start New Work's In the Cloud, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `start-new-work-goes-to-the-cloud` check
// copies modules/quickshell/launcher/Cloud.js, what it imports and
// tests/lib/Expect.js beside this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Cloud.js" as Cloud

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    function cases() {
        const words = { unnamed: "unnamed", unmachined: "unmachined", untaken: "untaken",
            unheard: "unheard" };
        const offered = [{ shape: "standard" }];

        // Joel, 2026-11-25: "we need full integation, from pi, from cli, from laucnher,
        // from website. i should not have to think to use this."
        Expect.same("the providers' shapes, a cost only where both its words are",
            Cloud.heard("", { offers: [{ shape: "standard", hourly: "0.38", currency: "USD" },
                { shape: "small", hourly: "0.1" }, { hourly: "1", currency: "USD" },
                { shape: "" }] }),
            { offers: [{ shape: "standard", hourly: "0.38", currency: "USD" },
                { shape: "small" }], unoffered: "", journal: null });
        Expect.same("none offered is an answer, so In the Cloud goes",
            Cloud.heard("why", { offers: [] }), { offers: [], unoffered: "", journal: null });
        Expect.same("a refusal keeps the last offers, its why journalled once",
            [Cloud.heard("", { error: { message: "no node" } }),
                Cloud.heard("no node", { error: { message: "no node" } }),
                Cloud.heard("", null).unoffered],
            [{ offers: null, unoffered: "no node", journal: "no node" },
                { offers: null, unoffered: "no node", journal: null },
                "the agents daemon isn't answering"]);
        Expect.same("a node that can't take the cloud's need takes back what it offered",
            Cloud.heard("", { error: { code: "untaken", message: "can't yet" } }),
            { offers: [], unoffered: "can't yet", journal: "can't yet" });
        // In the Cloud's drawer says the cheapest by the hour, the figure Joel reads
        // before a machine that costs money is made, so a new price is news.
        const standard = { shape: "standard", hourly: "0.38", currency: "USD" };
        Expect.same("offers differ by a shape, a price or a currency, as well as by count",
            [Cloud.differs([standard], [Object.assign({}, standard, { hourly: "0.52" })]),
                Cloud.differs([standard], [Object.assign({}, standard, { currency: "EUR" })]),
                Cloud.differs([standard], [Object.assign({}, standard, { shape: "large" })]),
                Cloud.differs([standard], [standard, { shape: "small" }]),
                Cloud.differs([standard], [{ shape: "standard" }]),
                Cloud.differs([standard], [Object.assign({}, standard)]),
                Cloud.differs([], [])],
            [true, true, true, true, true, false, false]);
        Expect.same("asked again only when never heard or ten minutes old",
            [Cloud.stale(0, 1000), Cloud.stale(1000, 1000 + 599999),
                Cloud.stale(1000, 1000 + 600000)], [true, false, true]);

        Expect.same("the cheapest by the hour, in the first priced one's currency",
            [Cloud.cheapest([{ shape: "a", hourly: "0.90", currency: "USD" },
                { shape: "b", hourly: "0.01", currency: "EUR" },
                { shape: "c", hourly: "0.38", currency: "USD" }]),
                Cloud.cheapest([{ shape: "a" }]), Cloud.cheapest([])],
            [{ shape: "c", hourly: "0.38", currency: "USD" }, null, null]);

        const checkout = { folder: "/home/j/src/core.nix", name: "~/src/core.nix",
            kind: "checkout" };
        const other = { folder: "/home/j/work/core.nix", name: "~/work/core.nix",
            kind: "checkout" };
        Expect.same("only places another machine knows by name, each once",
            Cloud.wheres([
                { folder: checkout.folder, name: "~/src/core.nix", start: checkout.folder },
                { folder: "/home/j/big/areas/system/pond", name: "//system/pond",
                    start: "//system/pond" },
                { folder: "/home/j/src/core.nix/modules", name: "~/src/core.nix/modules",
                    start: "/home/j/src/core.nix/modules" },
                { folder: other.folder, name: "~/work/core.nix", start: other.folder },
                { folder: "/home/j/src/-x", name: "~/src/-x", start: "/home/j/src/-x" }
            ], [checkout, other, { folder: "/home/j/src/-x", name: "-x", kind: "checkout" }]),
            [{ folder: checkout.folder, name: "~/src/core.nix", start: "core.nix" },
                { folder: "/home/j/big/areas/system/pond", name: "//system/pond",
                    start: "//system/pond" }]);
        Expect.same("a quest's folder the node doesn't call a checkout isn't named",
            Cloud.wheres([{ folder: "/home/j/notes", name: "~/notes",
                start: "/home/j/notes" }], []), []);

        Expect.same("work for the cloud needs the cloud, at a place by its name",
            [Cloud.ask(" fix it ", "core.nix", "cloud", offered),
                Cloud.ask("fix it", "//system/pond", "cloud", offered),
                Cloud.ask("https://github.com/o/r/issues/7", "", "cloud", offered)],
            [{ ask: "sprout", text: "fix it", needs: ["cloud"], folder: "core.nix" },
                { ask: "sprout", text: "fix it", needs: ["cloud"], folder: "//system/pond" },
                { ask: "sprout", text: "https://github.com/o/r/issues/7", needs: ["cloud"] }]);
        Expect.same("a link's machine goes as its name",
            Cloud.ask("https://github.com/o/r/issues/7", "", "skiff"),
            { ask: "sprout", text: "https://github.com/o/r/issues/7", on: "skiff" });
        Expect.same("nothing is asked of a place or a machine it couldn't say",
            [Cloud.ask("fix it", "/home/j/src/core.nix", "cloud", offered),
                Cloud.ask("fix it", "src/core.nix", "cloud", offered),
                Cloud.ask("fix it", "", "-"), Cloud.ask("fix it", "", "sk iff"),
                Cloud.ask("fix it", "", ""), Cloud.ask("  ", "core.nix", "cloud", offered)],
            [null, null, null, null, null, null]);
        Expect.same("and says which",
            [Cloud.refused("/home/j/src/core.nix", "cloud", offered, words),
                Cloud.refused("", "-", offered, words)],
            ["unnamed", "unmachined"]);
        // An old node would drop the need and start the work here, a link's start included.
        Expect.same("nothing is asked of the cloud while it offers nothing, and it says so",
            [Cloud.ask("fix it", "core.nix", "cloud", []),
                Cloud.ask("https://github.com/o/r/issues/7", "", Cloud.linked("on:cloud"), null),
                Cloud.refused("core.nix", "cloud", [], words)],
            [null, null, "untaken"]);
        // A link's cloud act may be Returned before the shell has heard any offers,
        // which take up to half a minute, so not yet heard is never said as can't.
        Expect.same("the cloud not heard yet says so, rather than that it can't be taken",
            [Cloud.ask("fix it", "core.nix", "cloud", null),
                Cloud.refused("core.nix", "cloud", null, words),
                Cloud.refused("/home/j/src/core.nix", "cloud", null, words)],
            [null, "unheard", "unnamed"]);
        Expect.same("a machine by its name needs no offer",
            Cloud.ask("https://github.com/o/r/issues/7", "", "skiff", []),
            { ask: "sprout", text: "https://github.com/o/r/issues/7", on: "skiff" });
        Expect.same("a link's on: the node's choice, the cloud, a machine, or nowhere",
            [Cloud.onOf(""), Cloud.onOf(" cloud "), Cloud.onOf("skiff"), Cloud.onOf("--all")],
            [{}, { needs: ["cloud"] }, { on: "skiff" }, null]);
        Expect.same("dots alone name no machine or place",
            [Cloud.onOf(".."), Cloud.ask("fix it", "..", "cloud", offered),
                Cloud.ask("fix it", ".", "cloud", offered)],
            [null, null, null]);
        // mycelium-link gives its `on` as it gives words and from, its name before it
        // (Launcher.qml's newWorkOn).
        Expect.same("a link's on is read past its name, and nowhere without it",
            [Cloud.linked("on:cloud"), Cloud.linked("on:skiff"), Cloud.linked("cloud", offered),
                Cloud.linked("on:"), Cloud.linked("")],
            ["cloud", "skiff", "-", "-", "-"]);
        Expect.same("so a link's cloud start needs the cloud",
            Cloud.ask("https://github.com/o/r/issues/7", "", Cloud.linked("on:cloud"), offered),
            { ask: "sprout", text: "https://github.com/o/r/issues/7", needs: ["cloud"] });
    }
}
