"""A VM's threads, as vm-first sees them: three at once, a fourth later.

The fourth starts once vm-first has had time to look, to show it puts a
thread started afterwards ahead too.
"""

import threading
import time

# Long enough to outlast the test, and how long vm-first has to look.
LIVES = 600
LOOKED = 4


def main() -> None:
    """Start three threads, then a fourth, and wait."""
    for _ in range(3):
        threading.Thread(target=time.sleep, args=(LIVES,)).start()
    time.sleep(LOOKED)
    threading.Thread(target=time.sleep, args=(LIVES,)).start()
    time.sleep(LIVES)


main()
