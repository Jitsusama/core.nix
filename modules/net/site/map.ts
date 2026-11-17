// Draws the network as floors: each eero's box with its radios and the
// devices on it, the switch beside it, and the gateway's floor with the
// modem and the internet. Everything is a role in the stylesheet, so the
// drawing follows the look.

import type { Channel, Device, Link, Network } from "./model.ts";
import { endAt, type Floor, linkBetween, type Shape } from "./shape.ts";
import { el, fitText, lookMark, text } from "./svg.ts";
import { drawWires, type Looks, looksOf, type Wire } from "./wires.ts";
import { ago, dbm, lasted, linkSpeed, plural, rate, roleWords, v4 } from "./words.ts";

/**
 * The drawing's grid, in pixels at zoom 1: a column for the floors' names,
 * one for the boxes, then the devices in columns of their own.
 */
const G = {
  left: 40,
  box: 200,
  boxW: 304,
  gap: 40,
  colW: 340,
  row: 24,
  bandTop: 24,
  bandEnd: 40,
  perColumn: 7,
  /** The narrowest the drawing is, however little it holds. */
  narrowest: 640,
} as const;

/** Inside a box: its words' left edge, its lines' baselines, its corners. */
const BOX = {
  pad: 16,
  title: 28,
  fact: 48,
  firstRow: 76,
  pitch: 20,
  head: 72,
  foot: 28,
  footLine: 16,
  round: 6,
} as const;

/** An eero's radio row: where each of its columns starts. */
const RADIO = { channel: 112, mark: 172, busy: 184, count: 288 } as const;

/** A switch's box, its eight ports as on its face, and its link's line. */
const SWITCH = {
  tall: 144,
  apart: 40,
  ports: 8,
  portStep: 34,
  portTop: 64,
  portW: 26,
  portH: 24,
  portRound: 3,
  lamp: { x: 7, y: 3, w: 12, h: 3, round: 1.5 },
  portName: { x: 13, y: 20 },
  linkLine: 112,
} as const;

/** A device's row: its ground, and where its name, address and signal sit. */
const ROW = {
  inset: 8,
  rise: 17,
  wide: 334,
  round: 3,
  nameRoom: 140,
  address: 148,
  mark: 282,
  figure: 318,
  headAbove: 22,
} as const;

/** The floor's name and count, and the modem and the internet below it. */
const FLOOR = { name: 48, count: 70 } as const;
const MODEM = {
  below: 48,
  wide: 480,
  tall: 136,
  columns: [16, 64, 216, 336],
  room: 32,
} as const;
const INTERNET = { ways: 56, mark: 4, markRoom: 14, ago: 100, line: 48, lineY: 22 } as const;

/** Where one device is drawn, and its group. */
interface Box {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
  readonly g: SVGGElement;
}

/** What a drawing made: where each device is, its size, and its marks' layer. */
interface Drawn {
  readonly at: ReadonlyMap<string, Box>;
  readonly width: number;
  readonly height: number;
  readonly marks: SVGGElement;
}

/** A link's speed, when its source gives one. */
const speedOf = (link: Link): string | undefined =>
  link.speedMbps ? linkSpeed(link.speedMbps) : undefined;

/** One sweep of the modem's channels in columns: its kinds, power and noise. */
function sweep(channels: readonly Channel[], word: string): string[] {
  const live = channels.filter((channel) => channel.active);
  const kinds = new Map<string, number>();
  for (const channel of live) {
    kinds.set(channel.kind, (kinds.get(channel.kind) ?? 0) + 1);
  }
  const power = live.flatMap((channel) => channel.powerDbmv ?? []);
  const snr = live.flatMap((channel) => channel.snrDb ?? []);
  const span = (values: number[], unit: string): string =>
    values.length > 0
      ? `${Math.min(...values).toFixed(1)}\u2013${Math.max(...values).toFixed(1)} ${unit}`
      : "";
  return [
    word,
    [...kinds].map(([kind, n]) => `${n} ${kind.toUpperCase()}`).join(", "),
    span(power, "dBmV"),
    snr.length > 0 ? `SNR ${span(snr, "dB")}` : "",
  ];
}

/** What each part of the drawing draws with, and what it's drawn into. */
class Pen {
  readonly network: Network;
  readonly shape: Shape;
  readonly looks: Looks;
  readonly draw: SVGSVGElement;
  readonly links: SVGGElement;
  readonly items: SVGGElement;
  readonly at = new Map<string, Box>();
  readonly wires: Wire[] = [];
  /** Now, in seconds since the epoch. */
  readonly now: number;

  constructor(draw: SVGSVGElement, network: Network, shape: Shape, now: number) {
    this.network = network;
    this.shape = shape;
    this.looks = looksOf(network);
    this.draw = draw;
    this.now = now;
    this.links = el("g", { class: "links" }, draw);
    this.items = el("g", { class: "items" }, draw);
  }

  /** A device's group, placed where `box` says. */
  item(device: Device, box: Omit<Box, "g">): SVGGElement {
    const attributes = { class: "n-item", "data-id": device.id, "data-level": "device" };
    const g = el("g", { ...attributes, "data-key": device.id }, this.items);
    this.at.set(device.id, { ...box, g });
    return g;
  }

  /** A box's ground and its title, with the facts line under it. */
  ground(g: SVGGElement, box: Omit<Box, "g">, title: string, facts: string): void {
    const { x, y, w, h } = box;
    el("rect", { x, y, width: w, height: h, rx: BOX.round, class: "n-box" }, g);
    text(g, title, { x: x + BOX.pad, y: y + BOX.title, kind: "n-title" });
    text(g, facts, { x: x + BOX.pad, y: y + BOX.fact, kind: "n-fact n-small" });
  }

  /** How many devices are on `point`'s radio `radio`. */
  onRadio(point: Device, radio: string): number {
    return this.network.links.filter(
      (link) =>
        link.b.device === point.id && link.b.radio === radio && !link.a.device.startsWith("eero:"),
    ).length;
  }

  /** One floor's eero, its radios and its way up; where its box ends. */
  point(point: Device, y: number): number {
    const h = BOX.head + point.radios.length * BOX.pitch + BOX.foot;
    const x = G.box;
    const g = this.item(point, { x, y, w: G.boxW, h });
    const facts = [roleWords(point), v4(point)[0]].filter(Boolean).join(" \u00b7 ");
    this.ground(g, { x, y, w: G.boxW, h }, point.model ?? point.name, facts);
    point.radios.forEach((radio, i) => {
      const ry = y + BOX.firstRow + i * BOX.pitch;
      const look = this.looks.busyRadio.has(`${point.id} ${radio.name}`);
      text(g, radio.name, { x: x + BOX.pad, y: ry, kind: "n-fact" });
      const channel = radio.channel === undefined ? "" : `ch ${radio.channel}`;
      text(g, channel, { x: x + RADIO.channel, y: ry, kind: "n-fact" });
      if (radio.busyPct !== undefined) {
        if (look) {
          lookMark(g, x + RADIO.mark, ry);
        }
        const kind = look ? "n-fact n-look on-box" : "n-fact";
        text(g, `${radio.busyPct}% busy`, { x: x + RADIO.busy, y: ry, kind });
      }
      const on = String(this.onRadio(point, radio.name));
      text(g, on, { x: x + RADIO.count, y: ry, kind: "n-figure", anchor: "end" });
    });
    text(g, this.wayUp(point), { x: x + BOX.pad, y: y + h - BOX.footLine, kind: "n-fact n-small" });
    return y + h;
  }

  /** How an eero reaches the one it hangs from, or its firmware and uptime. */
  wayUp(point: Device): string {
    const up = this.shape.via(point);
    const back = linkBetween(this.network, point.id, up);
    const upName = up === undefined ? undefined : this.shape.byId.get(up)?.name;
    if (back === undefined || up === undefined || upName === undefined || up === "modem") {
      const uptime = point.upS ? `up ${lasted(point.upS)}` : undefined;
      return [point.firmware, uptime].filter(Boolean).join(" \u00b7 ");
    }
    this.wires.push({ from: point.id, to: up, link: back });
    const channel = back.channel === undefined ? "" : `, ch ${back.channel}`;
    return `to ${upName} over ${back.medium === "wifi" ? `Wi-Fi${channel}` : "a cable"}`;
  }

  /** A switch beside its floor's eero, its ports as they stand; where it ends. */
  switchBox(device: Device, x: number, y: number): number {
    const h = SWITCH.tall;
    const g = this.item(device, { x, y, w: G.boxW, h });
    const facts = [device.model, v4(device)[0]].filter(Boolean).join(" \u00b7 ");
    this.ground(g, { x, y, w: G.boxW, h }, device.name, facts);
    const { lamp, portName } = SWITCH;
    device.ports.slice(0, SWITCH.ports).forEach((port, i) => {
      const px = x + BOX.pad + i * SWITCH.portStep;
      const py = y + SWITCH.portTop;
      const box = { x: px, y: py, width: SWITCH.portW, height: SWITCH.portH };
      el("rect", { ...box, rx: SWITCH.portRound, class: "n-port" }, g);
      // A linked port's lamp, as on the switch itself, so its state is told
      // by a shape rather than by how dark its edge is.
      if (port.link === "up") {
        const at = { x: px + lamp.x, y: py + lamp.y, width: lamp.w, height: lamp.h };
        el("rect", { ...at, rx: lamp.round, class: "n-lamp" }, g);
      }
      const name = { x: px + portName.x, y: py + portName.y, kind: "n-port-name" };
      text(g, port.name, name);
    });
    const up = this.shape.via(device);
    const toUp = linkBetween(this.network, device.id, up);
    if (toUp !== undefined && up !== undefined) {
      const mine = endAt(toUp, device.id);
      const theirs = this.shape.byId.get(endAt(toUp, up).device)?.name ?? up;
      const port = mine.port === undefined ? undefined : `port ${mine.port} to ${theirs}`;
      const words = [port, speedOf(toUp)].filter(Boolean).join(" \u00b7 ");
      text(g, words, { x: x + BOX.pad, y: y + SWITCH.linkLine, kind: "n-fact n-small" });
      this.wires.push({ from: device.id, to: up, link: toUp });
    }
    return y + h;
  }

  /** One device's row: its name, its address, and its signal or its port. */
  deviceRow(device: Device, x: number, y: number): void {
    const g = this.item(device, { x: x - ROW.inset, y: y - ROW.rise, w: ROW.wide, h: G.row });
    const ground = { x: x - ROW.inset, y: y - ROW.rise, width: ROW.wide, height: G.row };
    el("rect", { ...ground, rx: ROW.round, class: "n-row-ground" }, g);
    fitText(text(g, device.name, { x, y, kind: "n-name" }), device.name, ROW.nameRoom);
    text(g, v4(device)[0] ?? "", { x: x + ROW.address, y, kind: "n-fact" });
    const up = this.shape.via(device);
    const link = linkBetween(this.network, device.id, up);
    const figure = { x: x + ROW.figure, y, anchor: "end" };
    if (link?.signalDbm !== undefined) {
      const look = this.looks.weak.has(`${device.id} ${up ?? ""}`);
      if (look) {
        lookMark(g, x + ROW.mark, y);
      }
      text(g, dbm(link.signalDbm), { ...figure, kind: look ? "n-figure n-look" : "n-figure" });
    } else if (link?.medium === "ethernet" && up !== undefined) {
      const { port } = endAt(link, up);
      text(g, port === undefined ? "cable" : `port ${port}`, { ...figure, kind: "n-figure" });
    }
  }

  /** The devices on one floor, in columns; where they end, down and right. */
  deviceRows(devices: readonly Device[], x: number, top: number): { low: number; right: number } {
    const columns = devices.length > G.perColumn ? 2 : 1;
    const per = Math.max(1, Math.ceil(devices.length / columns));
    const signals = devices.some(
      (device) =>
        linkBetween(this.network, device.id, this.shape.via(device))?.signalDbm !== undefined,
    );
    for (let c = 0; signals && c < columns; c += 1) {
      const at = { x: x + c * G.colW + ROW.figure, y: top - ROW.headAbove, anchor: "end" };
      text(this.draw, "dBm", { ...at, kind: "n-fact n-small" });
    }
    devices.forEach((device, i) => {
      this.deviceRow(device, x + Math.floor(i / per) * G.colW, top + (i % per) * G.row);
    });
    return { low: top + per * G.row - ROW.inset, right: x + columns * G.colW };
  }

  /** The modem below the gateway's box, and the internet at its left; where it ends. */
  modem(modem: Device, gateway: Device, y: number): number {
    const facts = modem.modem;
    const { wide: w, tall: h } = MODEM;
    const g = this.item(modem, { x: G.box, y, w, h });
    const made = modem.vendor && modem.model ? `${modem.vendor} ${modem.model}` : undefined;
    const docsis = facts?.docsis ? `DOCSIS ${facts.docsis}` : undefined;
    const line = [made, docsis, v4(modem)[0]].filter(Boolean).join(" \u00b7 ");
    this.ground(g, { x: G.box, y, w, h }, modem.name, line);
    const sweeps = [sweep(facts?.downstream ?? [], "down"), sweep(facts?.upstream ?? [], "up")];
    sweeps.forEach((columns, i) => {
      const ry = y + BOX.firstRow + i * BOX.pitch;
      MODEM.columns.forEach((dx, k) => {
        text(g, columns[k] ?? "", { x: G.box + dx, y: ry, kind: "n-fact" });
      });
    });
    const lost = (facts?.downstream ?? []).reduce(
      (sum, each) => sum + (each.uncorrectable ?? 0),
      0,
    );
    const uptime = modem.upS ? `up ${lasted(modem.upS)}` : undefined;
    const foot = [uptime, plural(lost, "uncorrectable codeword")].filter(Boolean).join(" \u00b7 ");
    text(g, foot, { x: G.box + BOX.pad, y: y + h - BOX.footLine, kind: "n-fact n-small" });
    const cable = linkBetween(this.network, modem.id, gateway.id);
    if (cable !== undefined) {
      this.wires.push({ from: gateway.id, to: modem.id, link: cable, down: true });
    }
    this.internet(y);
    return y + h;
  }

  /** The internet, at the far left of the modem, as eero last measured it. */
  internet(y: number): void {
    const { internet } = this.network;
    if (internet === undefined) {
      return;
    }
    const g = el("g", { class: "internet" }, this.draw);
    text(g, internet.provider ?? "The internet", { x: G.left, y: y + BOX.title, kind: "n-title" });
    const ways: [string, number | undefined][] = [
      ["down", internet.downMbps],
      ["up", internet.upMbps],
    ];
    ways.forEach(([way, value], k) => {
      if (value === undefined) {
        return;
      }
      const ry = y + INTERNET.ways + k * BOX.pitch;
      const look = this.looks.ceiling.has(way);
      if (look) {
        lookMark(g, G.left + INTERNET.mark, ry);
      }
      const at = { x: G.left + (look ? INTERNET.markRoom : 0), y: ry };
      text(g, `${way} ${rate(value)}`, { ...at, kind: look ? "n-fact n-look" : "n-fact" });
    });
    if (internet.measuredAt !== undefined) {
      const when = ago(internet.measuredAt, this.now);
      text(g, when, { x: G.left, y: y + INTERNET.ago, kind: "n-fact n-small" });
    }
    const d = `M${G.box - INTERNET.line} ${y + INTERNET.lineY}H${G.box}`;
    el("path", { d, class: "n-link", "data-way": "internet", "data-line": "" }, this.links);
  }

  /** One floor in its band from `top`: its name, its boxes and its devices. */
  floor(floor: Floor, top: number): { low: number; right: number } {
    const { point } = floor;
    const rule = { x1: G.left, x2: 0, y1: top + 0.5, y2: top + 0.5 };
    el("line", { ...rule, class: "n-floor", "data-rule": "" }, this.draw);
    text(this.draw, point.name, { x: G.left, y: top + FLOOR.name, kind: "n-floor-name" });
    const count = plural(floor.devices.length + floor.switches.length, "device");
    text(this.draw, count, { x: G.left, y: top + FLOOR.count, kind: "n-fact" });
    const boxY = top + G.bandTop;
    let low = this.point(point, boxY);
    let x = G.box + G.boxW + G.gap;
    for (const device of floor.switches) {
      low = Math.max(low, this.switchBox(device, x, boxY));
      x += G.boxW + SWITCH.apart;
    }
    const rows = this.deviceRows(floor.devices, x, boxY + BOX.title);
    low = Math.max(low, rows.low);
    let { right } = rows;
    // The gateway's floor holds the modem and the internet below its box.
    if (this.shape.modem !== undefined && point.id === this.shape.gateway?.id) {
      low = this.modem(this.shape.modem, point, low + MODEM.below);
      right = Math.max(right, G.box + MODEM.wide + MODEM.room);
    }
    return { low, right };
  }
}

/** Draws `network` into `draw`, laid out as `shape` says, at `now` in seconds. */
function drawMap(draw: SVGSVGElement, network: Network, shape: Shape, now: number): Drawn {
  draw.replaceChildren();
  const pen = new Pen(draw, network, shape, now);
  const marks = el("g", { class: "marks" }, draw);
  let y = 0;
  let width = 0;
  for (const floor of shape.floors) {
    const { low, right } = pen.floor(floor, y);
    width = Math.max(width, right);
    y = low + G.bandEnd;
  }
  drawWires({ shape, looks: pen.looks, links: pen.links, at: pen.at }, pen.wires, G.box);
  width = Math.max(width, G.narrowest) + G.left;
  for (const rule of draw.querySelectorAll("[data-rule]")) {
    rule.setAttribute("x2", String(width - G.left));
  }
  draw.setAttribute("viewBox", `0 0 ${width} ${y}`);
  return { at: pen.at, width, height: y, marks };
}

export type { Box, Drawn };
export { drawMap };
