// A clock a test moves by hand in place of the browser's, so a wait to try
// again passes when the test says and not when the machine does.

import type { Clock } from "../feed.ts";

/** A clock a test moves by hand, holding what waits on it. */
class Hands implements Clock {
  at = 1_000_000;
  readonly waiting: { readonly at: number; readonly then: () => void }[] = [];

  readonly now = (): number => this.at;

  readonly later = (milliseconds: number, then: () => void): (() => void) => {
    const waits = { at: this.at + milliseconds, then };
    this.waiting.push(waits);
    return () => this.waiting.splice(this.waiting.indexOf(waits), 1);
  };

  /** Moves the clock on `milliseconds`, running what waited till then. */
  pass(milliseconds: number): void {
    this.at += milliseconds;
    for (const waits of this.waiting.filter((each) => each.at <= this.at)) {
      this.waiting.splice(this.waiting.indexOf(waits), 1);
      waits.then();
    }
  }
}

export { Hands };
