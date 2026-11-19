// Forage records as the mycelium node tells them, for the tests that read
// them: one forage:<cell> record for each cell of this machine that works
// on a quest.

import { Held } from "../../../local-sites/kit/records.ts";

const plan = {
  id: "PLAN-20261116-HOVERS",
  kind: "plan",
  title: "Hover Over the Field",
  stage: "build",
  ticked: 2,
  open: 3,
};
const report = {
  id: "REPT-20261116-LANDED",
  kind: "report",
  title: "Where It Landed",
  stage: "concluded",
  ticked: 1,
  open: 0,
};

/** A forage record for `cell`, on kestrel, with `more` over its fields. */
function forage(cell: string, more: { readonly [field: string]: unknown } = {}) {
  return {
    key: `forage:${cell}`,
    kind: "forage",
    heard: 0,
    cell,
    session: `0000-${cell.toLowerCase()}`,
    machine: "kestrel",
    quest: `QEST-20261116-${cell}`,
    purpose: `${cell} learns to hover over one field.`,
    status: "active",
    documents: [report, plan],
    lineage: { started_by: null, parent: null },
    ...more,
  };
}

/** A watch told `records` whole. */
function told(records: readonly { readonly key: string }[]): Held {
  const held = new Held();
  held.took({ records });
  return held;
}

export { forage, plan, report, told };
