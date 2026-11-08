// What a notice's body says, as NoticeBody.js reads it: plain words, styled
// text with only the spec's markup, and its links, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-notice-keeps-its-links` check copies NoticeBody.js beside this file and
// runs `qml -platform offscreen notice-body.qml`.
import QtQuick
import "NoticeBody.js" as NoticeBody

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
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        // Plain words stay as they are, and nothing is a link.
        expect("plain words", NoticeBody.read("Lunch at noon?"),
            { plain: "Lunch at noon?", styled: "Lunch at noon?", links: [] });
        // The spec's markup passes, and its entities read as letters.
        expect("bold and entities", NoticeBody.read("<b>Build</b> passed &amp; merged"),
            { plain: "Build passed & merged", styled: "<b>Build</b> passed &amp; merged",
                links: [] });
        // A link keeps its words and where it goes, in the order they come.
        expect("a link", NoticeBody.read('See <a href="https://github.com/x/y/pull/3">#3</a>'),
            { plain: "See #3", styled: 'See <a href="https://github.com/x/y/pull/3">#3</a>',
                links: [{ text: "#3", href: "https://github.com/x/y/pull/3" }] });
        expect("two links, in order", NoticeBody.read(
            "<a href='https://a.example/1'>one</a> and <a href=https://b.example/2>two</a>")
            .links.map(link => link.href), ["https://a.example/1", "https://b.example/2"]);
        // A bare address is a link too, without the full stop after it.
        expect("a bare address", NoticeBody.read("Docs at https://docs.google.com/d/abc."),
            { plain: "Docs at https://docs.google.com/d/abc.",
                styled: 'Docs at <a href="https://docs.google.com/d/abc">'
                    + "https://docs.google.com/d/abc</a>.",
                links: [{ text: "https://docs.google.com/d/abc",
                    href: "https://docs.google.com/d/abc" }] });
        // Anything but the web or mail is said as words and never opened.
        expect("a script is no link", NoticeBody.read('<a href="javascript:alert(1)">x</a>'),
            { plain: "x", styled: "x", links: [] });
        // Any other tag, or a bare <, is the sender's words, escaped.
        expect("other tags escaped", NoticeBody.read('<font color="red">hi</font> a < b'),
            { plain: '<font color="red">hi</font> a < b',
                styled: "&lt;font color=&quot;red&quot;&gt;hi&lt;/font&gt; a &lt; b",
                links: [] });
        // A picture in the words is said by what it shows, never fetched.
        expect("an image's words", NoticeBody.read('<img src="https://x/y.png" alt="a cat">'),
            { plain: "a cat", styled: "a cat", links: [] });
        // Line breaks are kept: plain as the sender's, styled as breaks.
        expect("line breaks", NoticeBody.read("one\ntwo<br/>three"),
            { plain: "one\ntwo\nthree", styled: "one<br>two<br>three", links: [] });
        // A tag left open is closed at the end, and a stray close dropped.
        expect("tags made whole", NoticeBody.read("<b><i>loud</b> quiet</u>").styled,
            "<b><i>loud</i></b> quiet");
        // An empty body, or none, is nothing.
        expect("no body", NoticeBody.read(undefined), { plain: "", styled: "", links: [] });

        // A link said short: its host, or a mail's address.
        expect("a host", NoticeBody.host("https://www.GitHub.com/x?y=1"), "github.com");
        expect("a mail", NoticeBody.host("mailto:joel%40example.com?subject=hi"),
            "joel@example.com");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
