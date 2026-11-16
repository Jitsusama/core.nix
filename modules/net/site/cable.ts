// The modem's cable in its panel: its channels, as its status pages give
// them, and what each lost.

import { type Cell, type Child, facts, h, table } from "./elements.ts";
import type { Channel, Device } from "./model.ts";
import { lasted } from "./words.ts";

/** The wide channels DOCSIS 3.1 added, which sort after the narrow ones. */
const WIDE = /^ofdm/u;

/**
 * Channels in number order, single-carrier first, since the modem lists
 * them in the order it locked on.
 */
function ordered(list: readonly Channel[]): Channel[] {
  const wide = (channel: Channel): number => Number(WIDE.test(channel.kind));
  return [...list].sort((x, y) => wide(x) - wide(y) || x.channel - y.channel);
}

/** A count of what was lost, gold once it's any. */
function lost(n: number | undefined): Cell {
  if (n === undefined) {
    return "";
  }
  return n > 0 ? { text: String(n), look: true } : n;
}

/**
 * The downstream, with what each channel lost over the day's readings once
 * net keeps a day: the column is left out until then rather than shown
 * blank.
 */
function downstream(channels: readonly Channel[], lately: boolean): HTMLElement {
  const head = ["ch", "kind", "dBmV", "SNR", "lost", ...(lately ? ["lately"] : [])];
  const rows = ordered(channels).map((channel) => [
    channel.channel,
    channel.kind,
    channel.powerDbmv?.toFixed(1) ?? "",
    channel.snrDb?.toFixed(1) ?? "",
    lost(channel.uncorrectable),
    ...(lately ? [lost(channel.lostLately)] : []),
  ]);
  const nums = lately ? [0, 2, 3, 4, 5] : [0, 2, 3, 4];
  return table(head, rows, nums);
}

/** The upstream, which gives no noise figure, so its table has no column for one. */
function upstream(channels: readonly Channel[]): HTMLElement {
  const rows = ordered(channels).map((channel) => [
    channel.channel,
    channel.kind,
    channel.powerDbmv?.toFixed(1) ?? "not given",
  ]);
  return table(["ch", "kind", "dBmV"], rows, [0, 2]);
}

/** The modem's cable, for a device that is one. */
function cable(device: Device): Child[] {
  const { modem } = device;
  if (modem === undefined) {
    return [];
  }
  const { latelyS } = modem;
  const lately =
    latelyS !== undefined && modem.downstream.some((channel) => channel.lostLately !== undefined);
  return [
    h("h3", {}, "The Cable"),
    facts([
      ["DOCSIS", modem.docsis, true],
      ["internet access", modem.networkAccess],
      ["its address with the provider", modem.address, true],
    ]),
    h("h3", {}, "Downstream"),
    downstream(modem.downstream, lately),
    h(
      "p",
      { class: "hint" },
      "Lost counts the pieces of data a channel got too damaged to repair since the modem ",
      "started; one channel far above the rest usually means noise on that one frequency. ",
      lately && latelyS !== undefined
        ? `Lately is what it lost in the last ${lasted(latelyS)}. `
        : "",
      "Each kind numbers its own channels: sc-qam down and atdma up are the older narrow ones, ",
      "ofdm and ofdma the wide ones DOCSIS 3.1 added. ",
      "dBmV is the signal's strength and SNR its clearness over noise, in dB.",
    ),
    h("h3", {}, "Upstream"),
    upstream(modem.upstream),
  ];
}

export { cable };
