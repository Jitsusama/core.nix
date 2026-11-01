// What each opening of the launcher did, as a trace in OpenTelemetry's
// model: the opening is the root span, and each step in, step back, act and
// search inside it is a span of its own, ended once its list has settled,
// with where the list laid its first row (Joel, 2026-10-31: "you really
// need to get proper observability working, including tracing for stuff
// like this", of a void above a menu's first row). Nothing is written per
// key: spans are kept in memory, a fixed number of them, and each opening
// is sent whole as it closes (Traces.qml). Apart from the launcher, so
// tests/traces.qml can hold it in Qt's own engine.
.pragma library

// How many hex digits OTLP's JSON mapping gives a trace's id and a span's.
const TRACE_DIGITS = 32;
const SPAN_DIGITS = 16;
// Ids are made four hex digits, sixteen bits, at a time.
const HEX = 16;
const CHUNK_DIGITS = 4;
const CHUNK = HEX ** CHUNK_DIGITS;

// A random id of `digits` hex digits.
function id(digits) {
    let made = "";
    while (made.length < digits) {
        made += Math.floor(Math.random() * CHUNK).toString(HEX).padStart(CHUNK_DIGITS, "0");
    }
    return made.slice(0, digits);
}

// A span begun at `at`, in milliseconds since the epoch, under `parent`
// (a span, or null for an opening's root), with what it's about.
function begun(name, at, parent, about) {
    return {
        trace: parent ? parent.trace : id(TRACE_DIGITS),
        span: id(SPAN_DIGITS),
        parent: parent ? parent.span : "",
        name,
        start: at,
        end: null,
        about: Object.assign({}, about ?? {})
    };
}

// The span ended at `at`, with more of what it was about.
function ended(span, at, about) {
    return Object.assign({}, span, { end: at, about: Object.assign({}, span.about, about ?? {}) });
}

// The spans kept, newest last, holding at most `most`: the oldest go first.
function kept(spans, span, most) {
    const all = spans.concat([span]);
    return all.length > most ? all.slice(all.length - most) : all;
}

// Whether a list settled with space above its first row, the void Joel saw:
// its first row drawn below the list's top. Scrolled down, the first row is
// above the top, which is no void.
function voided(about) {
    return typeof about?.firstAt === "number" && about.firstAt > 0;
}

// One attribute in OTLP's JSON mapping: a whole number as intValue, which
// the mapping writes as a string, a fraction as doubleValue, a truth as
// boolValue and anything else as its words.
function attribute(key, value) {
    if (typeof value === "number" && Number.isInteger(value)) {
        return { key, value: { intValue: String(value) } };
    }
    if (typeof value === "number") {
        return { key, value: { doubleValue: value } };
    }
    if (typeof value === "boolean") {
        return { key, value: { boolValue: value } };
    }
    return { key, value: { stringValue: String(value ?? "") } };
}

// Milliseconds since the epoch as OTLP's nanoseconds, a string of digits.
function nanos(ms) {
    return `${Math.round(ms)}000000`;
}

// The ended spans as one ExportTraceServiceRequest, in OTLP's JSON mapping,
// from the service "launcher": one line for the collector. A span not yet
// ended is left out.
function request(spans) {
    const done = spans.filter(span => span.end !== null);
    return {
        resourceSpans: [{
            resource: { attributes: [attribute("service.name", "launcher")] },
            scopeSpans: [{
                scope: { name: "launcher" },
                spans: done.map(span => Object.assign({
                    traceId: span.trace,
                    spanId: span.span,
                    name: span.name,
                    kind: 1,
                    startTimeUnixNano: nanos(span.start),
                    endTimeUnixNano: nanos(span.end),
                    attributes: Object.keys(span.about).map(key => attribute(key, span.about[key]))
                }, span.parent === "" ? {} : { parentSpanId: span.parent }))
            }]
        }]
    };
}
