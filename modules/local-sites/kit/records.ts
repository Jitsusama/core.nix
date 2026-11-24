// The records a page holds of one watch, folded as a daemon tells them, and
// the guards a page reads a record's fields through.
//
// A watch answers every record whole, then tells only what changed, by key,
// and the keys that went (docs/daemons.md). Held folds those the way
// daemon-serve's own watch module does, so a page holds what the launcher
// holds: a record told twice is one record, a field told null is gone, and
// a field the feed asks deep is mended one level in rather than replaced.
//
// A record is untyped JSON from another program, so nothing here trusts a
// field's shape: each guard answers the field when it is what it should be,
// and undefined when it isn't, and the page says what it couldn't read.

/** A JSON object, read as nothing more. */
interface Fields {
  readonly [field: string]: unknown;
}

/** A record as the daemon interface says one: an identity key and fields. */
interface Record extends Fields {
  readonly key: string;
}

/**
 * What one answer moved: the keys it told, the keys that went, and whether it
 * told every record, as a watch's first answer does, rather than what changed.
 * Only a whole answer makes the records held complete again after a line the
 * page couldn't take, since that line may have been a change nothing repeats.
 */
interface Moved {
  readonly changed: readonly string[];
  readonly gone: readonly string[];
  readonly whole: boolean;
}

/** What one answer moved, and how much of it wasn't a record. */
interface Taken extends Moved {
  readonly skipped: number;
}

/** Whether `value` is a JSON object, and not an array or null. */
function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Whether `value` is a record: an object with a string key. */
function isRecord(value: unknown): value is Record {
  return isObject(value) && typeof value["key"] === "string";
}

/** The field `field` of `record` when it is a string. */
function text(record: Fields, field: string): string | undefined {
  const value = record[field];
  return typeof value === "string" ? value : undefined;
}

/** The field `field` of `record` when it is a finite number. */
function number(record: Fields, field: string): number | undefined {
  const value = record[field];
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

/** The field `field` of `record` when it is a list of strings. */
function texts(record: Fields, field: string): readonly string[] | undefined {
  const value = record[field];
  if (!Array.isArray(value)) {
    return undefined;
  }
  const strings = value.filter((each): each is string => typeof each === "string");
  return strings.length === value.length ? strings : undefined;
}

/** The field `field` of `record` when it is an object. */
function object(record: Fields, field: string): Fields | undefined {
  const value = record[field];
  return isObject(value) ? value : undefined;
}

/** `fields` mended into `into`: each field replaced, and one told null gone. */
function mended(into: Fields, fields: Fields, deep: ReadonlySet<string>): Fields {
  const record: { [field: string]: unknown } = { ...into };
  for (const [field, value] of Object.entries(fields)) {
    const held = record[field];
    if (value === null) {
      delete record[field];
    } else if (deep.has(field) && isObject(held) && isObject(value)) {
      record[field] = mended(held, value, new Set());
    } else {
      record[field] = value;
    }
  }
  return record;
}

/** `value` as a list, or none when it isn't one. */
function listed(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : [];
}

/** The records of one watch, by key, as its answers have told them. */
class Held {
  readonly #records = new Map<string, Record>();
  readonly #deep: ReadonlySet<string>;

  /** Records held of a watch whose `deep` fields are told one level in. */
  constructor(deep: readonly string[] = []) {
    this.#deep = new Set(deep);
  }

  /** How many records are held. */
  get size(): number {
    return this.#records.size;
  }

  /** The record held for `key`. */
  get(key: string): Record | undefined {
    return this.#records.get(key);
  }

  /** Every record held, in the order each was first told. */
  all(): readonly Record[] {
    return [...this.#records.values()];
  }

  /**
   * Takes one answer to the watch: its records whole, which stand for every
   * record it holds, those it no longer names gone; then each change mended
   * into the record it names; then the keys gone. What isn't a record in it
   * is passed over and counted in `skipped`, for the page to say.
   */
  took(answer: Fields): Taken {
    const changed: string[] = [];
    const gone: string[] = [];
    let skipped = 0;
    const records = answer["records"];
    if (Array.isArray(records)) {
      skipped += this.#whole(records, changed, gone);
    }
    skipped += this.#mend(listed(answer["changed"]), changed);
    for (const key of listed(answer["gone"])) {
      if (typeof key === "string" && this.#records.delete(key)) {
        gone.push(key);
      }
    }
    return { changed, gone, whole: Array.isArray(records), skipped };
  }

  /** Takes `records` as every record, saying each, and those gone. */
  #whole(records: readonly unknown[], changed: string[], gone: string[]): number {
    const told = records.filter(isRecord);
    const named = new Set(told.map((record) => record.key));
    for (const key of this.#records.keys()) {
      if (!named.has(key)) {
        this.#records.delete(key);
        gone.push(key);
      }
    }
    for (const record of told) {
      this.#records.set(record.key, record);
      changed.push(record.key);
    }
    return records.length - told.length;
  }

  /** Mends each of `changes` into its record, saying each. */
  #mend(changes: readonly unknown[], changed: string[]): number {
    const told = changes.filter(isRecord);
    for (const change of told) {
      const held = this.#records.get(change.key) ?? { key: change.key };
      this.#records.set(change.key, { ...mended(held, change, this.#deep), key: change.key });
      changed.push(change.key);
    }
    return changes.length - told.length;
  }
}

export type { Fields, Moved, Record, Taken };
export { Held, isObject, isRecord, number, object, text, texts };
