// A stand-in People source for the desktop's VM tests whose people the
// people daemon holds, as Tuple's are: the daemon runs people-roster.sh
// beside it, which says a crowd of 8,000, and People reads from the
// daemon only those it lists, none here, and those a search finds.
import qs
import QtQuick

PersonSource {
    daemonHolds: true
}
