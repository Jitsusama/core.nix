# The Sound VM Tests

What sound does on a running machine, booted in QEMU: calls noticed and
written down, dictation hearing its chord and its microphone, and each
microphone and speaker tuned as its module says. A VM has no sound
hardware, so each test's opening comment says what stands in for it.

| File                                     | Holds                                                     |
| ---------------------------------------- | --------------------------------------------------------- |
| `calls.nix`, `-vm-test.py`               | root telling whether an account's PipeWire holds a call   |
| `call-transcripts.nix`, `-vm-test.py`    | a call heard and written down, each side in its turn      |
| `stand-in-summarizer.py`                 | that test's stand-in for the model that sums a call up    |
| `dictation-chord.nix`, `-vm-test.py`     | dictation's chord heard held and tapped, and nothing else |
| `dictation-listening.nix`, `-vm-test.py` | dictation saying it listens only once sound arrives       |
| `noise-cancelled.nix`, `-vm-test.py`     | a microphone offered again with its noise taken out       |
| `microphones.nix`, `-vm-test.py`         | a webcam's microphone ranked below a USB microphone's     |
| `shure-mv7-voice.nix`, `-vm-test.py`     | the MV7's voice filter raising speech and capping peaks   |
| `pebble-x-plus.nix`, `-vm-test.py`       | the Pebble X Plus's tuning, also after a replug           |
| `speakers.nix`, `-vm-test.py`            | the XPS 14's speaker tuning, and the headphones untouched |

The modules' own checks, which need no VM, sit beside each module under
`../../../modules/`.
