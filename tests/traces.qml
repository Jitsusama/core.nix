// How the launcher traces each opening, run in Qt's own JavaScript engine,
// the one the shell runs it in. The `the-launcher-traces-each-opening`
// check copies modules/quickshell/launcher/Traces.js beside this file and
// runs `qml -platform offscreen traces.qml`. Each case is what the
// collector and a reader of a void lean on: an opening's spans share its
// trace and name it their parent, the oldest let go, a void told from a
// list scrolled down, and OTLP's JSON mapping as the collector reads it.
import QtQuick
import "Traces.js" as Traces

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        const expect = (what, holds, said) => {
            if (!holds)
                failures.push(`${what}: ${said}`);
        };

        // An opening at 1000, a step into Machine at 1100 settled at 1150,
        // closed at 2000.
        const opening = Traces.begun("open", 1000, null, {});
        const step = Traces.begun("step", 1100, opening, { into: "category:computer", depth: 0 });
        const settled = Traces.ended(step, 1150, { firstAt: 0, contentY: 0 });
        const closed = Traces.ended(opening, 2000, { depth: 1 });
        expect("a trace id of 32 hex digits", /^[0-9a-f]{32}$/.test(opening.trace), opening.trace);
        expect("a span id of 16 hex digits", /^[0-9a-f]{16}$/.test(step.span), step.span);
        expect("a step in its opening's trace", step.trace === opening.trace, step.trace);
        expect("its opening its parent", step.parent === opening.span, step.parent);
        expect("ending keeps what it began with", settled.about.into === "category:computer"
            && settled.about.firstAt === 0, JSON.stringify(settled.about));
        expect("ending copies", step.end === null && step.about.firstAt === undefined,
            JSON.stringify(step));
        expect("two openings two traces", Traces.begun("open", 0, null, {}).trace !== opening.trace,
            "the same");

        // The oldest go first.
        let spans = [];
        for (let at = 0; at < 5; at += 1)
            spans = Traces.kept(spans, Traces.ended(Traces.begun(`s${at}`, at, null, {}), at, {}), 3);
        expect("three kept, newest last", spans.map(span => span.name).join(" ") === "s2 s3 s4",
            spans.map(span => span.name).join(" "));

        // Space above the first row is the void; above the top is scrolled.
        expect("a void", Traces.voided({ firstAt: 48 }), "not one");
        expect("at rest", !Traces.voided({ firstAt: 0 }), "a void");
        expect("scrolled down", !Traces.voided({ firstAt: -96 }), "a void");
        expect("not drawn", !Traces.voided({ firstAt: null }), "a void");

        // As the collector reads it: one resource, the service named, ids
        // in hex, times in nanoseconds as strings, and a span not ended
        // left out.
        const request = Traces.request([closed, settled, Traces.begun("search", 1200, opening, {})]);
        const resource = request.resourceSpans[0];
        const service = resource.resource.attributes[0];
        expect("the service", service.key === "service.name"
            && service.value.stringValue === "launcher", JSON.stringify(service));
        const sent = resource.scopeSpans[0].spans;
        expect("ended spans alone", sent.length === 2, sent.length);
        const root = sent[0];
        const child = sent[1];
        expect("nanoseconds as strings", root.startTimeUnixNano === "1000000000"
            && root.endTimeUnixNano === "2000000000", JSON.stringify(root));
        expect("a root has no parent", !("parentSpanId" in root), JSON.stringify(root));
        expect("a child names its parent", child.parentSpanId === root.spanId
            && child.traceId === root.traceId, JSON.stringify(child));
        const about = {};
        for (const each of child.attributes)
            about[each.key] = each.value;
        expect("a whole number as intValue", about.firstAt?.intValue === "0",
            JSON.stringify(about.firstAt));
        expect("words as stringValue", about.into?.stringValue === "category:computer",
            JSON.stringify(about.into));
        expect("one line", !JSON.stringify(request).includes("\n"), "more than one");
        expect("a truth", Traces.attribute("b", false).value.boolValue === false, "not one");
        expect("a fraction", Traces.attribute("f", 1.5).value.doubleValue === 1.5, "not one");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
            return;
        }
        console.info("every case held");
        Qt.exit(0);
    }
}
