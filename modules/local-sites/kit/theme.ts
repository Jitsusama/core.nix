// The theme switch every site carries in its head (switch.css draws it):
// light, the system's, or dark, kept across visits. The kit's early.js, in
// the page's head, sets the kept choice before the first paint; this checks
// its radio and keeps what Joel turns it to. The radios are real ones, so
// their keys are the browser's own.

type Choice = "light" | "system" | "dark";

const CHOICES: readonly Choice[] = ["light", "system", "dark"];

/** Whether `value` is one of the switch's choices. */
function isChoice(value: string | undefined): value is Choice {
  return CHOICES.some((choice) => choice === value);
}

/** Keeps `choice` for the next visit, where the browser keeps anything. */
function keep(choice: Choice): void {
  try {
    if (choice === "system") {
      localStorage.removeItem("theme");
    } else {
      localStorage.setItem("theme", choice);
    }
  } catch {
    // A browser that keeps nothing, as a private window may refuse to,
    // still switches for this visit, which is all it can be asked for.
  }
}

/** Shows the page in `choice`. */
function wear(choice: Choice): void {
  if (choice === "system") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.dataset["theme"] = choice;
  }
}

/** Wires the switch's radios in `document`, its kept choice checked. */
function themeSwitch(): void {
  const kept = document.documentElement.dataset["theme"];
  const now: Choice = isChoice(kept) ? kept : "system";
  for (const radio of document.querySelectorAll<HTMLInputElement>(".switch input[type=radio]")) {
    radio.checked = radio.value === now;
    radio.addEventListener("change", () => {
      if (radio.checked && isChoice(radio.value)) {
        wear(radio.value);
        keep(radio.value);
      }
    });
  }
}

export { themeSwitch };
