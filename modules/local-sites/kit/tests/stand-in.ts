// A socket a test drives in place of the browser's WebSocket: what the page
// sent it, and lines for it to tell the page.

import type { Socket } from "../feed.ts";

/** A socket a test drives: what the page sent, and lines to tell it. */
class StandIn implements Socket {
  onopen: (() => void) | null = null;
  onmessage: ((event: { readonly data: unknown }) => void) | null = null;
  onclose: (() => void) | null = null;
  readonly sent: unknown[] = [];
  readonly url: string;
  closed = false;

  constructor(url: string) {
    this.url = url;
  }

  send(line: string): void {
    this.sent.push(JSON.parse(line));
  }

  close(): void {
    this.closed = true;
  }

  /** Tells the page `line`, as JSON unless it is already text. */
  tell(line: unknown): void {
    this.onmessage?.({ data: typeof line === "string" ? line : JSON.stringify(line) });
  }
}

export { StandIn };
