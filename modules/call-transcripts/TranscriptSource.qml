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
}
