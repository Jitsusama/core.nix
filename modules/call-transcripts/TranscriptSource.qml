// The calls written down as one of People's sources (PersonSource.qml):
// someone's Last Met lists each call with them, by the address
// call-transcripts keeps of who a Tuple call was with, each the call's
// one row (Transcripts.row), newest first among the meetings they had.
// A Tuple call is the two of them. It knows nobody of its own, since
// everyone a call was with is someone Tuple knows already.
import qs
import QtQuick

PersonSource {
    function recent(person, provider) {
        return Transcripts.withThem(person.emails)
            .map(call => ({ at: call.started, row: Transcripts.row(call, provider), kind: "met",
                one: true }));
    }

    // The first of each for their drawer: the newest call, named as its
    // row is, the one row made (PersonSource's firsts).
    function firsts(person, provider) {
        const newest = Transcripts.withThem(person.emails)
            .reduce((last, call) => last && last.started >= call.started ? last : call, null);
        return {
            met: newest ? { at: newest.started, name: Transcripts.row(newest, provider).name,
                one: true } : null,
            wait: null
        };
    }
}
