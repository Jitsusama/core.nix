// How wide words are as a Text draws them, asked from a binding. A
// FontMetrics measures as native text is laid, where a Text draws with
// Qt's own rendering, so where hinting is full at 1 it read a line of the
// serif 396 pixels wide that Qt drew 385 (2026-11-15); only a TextMetrics,
// whose render type is a Text's, measures as it's drawn. It's measured by
// setting its words, which a binding reading the measure would be told of
// at every change, and loop, so its own handler measures them: what a
// handler reads no binding follows, and the width comes back in a plain
// object it holds. A binding reading widthOf is made again only when it
// reads the face, as Measures.js's drawnMeasurer does.
import QtQuick

TextMetrics {
    id: measure

    // Where the handler leaves the width it measured, read once it has.
    readonly property var answer: ({ wide: 0 })

    signal asked(string words)

    onAsked: words => {
        measure.text = words;
        measure.answer.wide = measure.advanceWidth;
    }

    // How wide `words` are drawn in its face.
    function widthOf(words: string): real {
        asked(words);
        return answer.wide;
    }
}
