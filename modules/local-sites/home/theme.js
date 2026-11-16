// theme: the head's three-way switch, light, as the system is, or dark. The head's own
// script applies a stored choice before first paint; this keeps the switch in step with it.
(() => {
  // A browser that keeps nothing (a private window, storage turned off)
  // still switches; the choice just isn't kept for the next visit.
  const store = {
    set: (key, value) => {
      try {
        localStorage.setItem(key, value);
      } catch {
        // Kept for this visit alone.
      }
    },
    del: (key) => {
      try {
        localStorage.removeItem(key);
      } catch {
        // Nothing was kept to forget.
      }
    },
  };
  const themeSwitch = document.querySelector(".switch");
  const steps = new Map([
    ["ArrowLeft", -1],
    ["ArrowUp", -1],
    ["ArrowRight", 1],
    ["ArrowDown", 1],
  ]);
  const showTheme = () => {
    const choice = document.documentElement.dataset.theme || "system";
    for (const b of themeSwitch.querySelectorAll("button")) {
      const on = b.dataset.choice === choice;
      b.setAttribute("aria-checked", String(on));
      b.tabIndex = on ? 0 : -1;
    }
  };
  const chooseTheme = (choice) => {
    if (choice === "system") {
      delete document.documentElement.dataset.theme;
      store.del("theme");
    } else {
      document.documentElement.dataset.theme = choice;
      store.set("theme", choice);
    }
    showTheme();
  };
  showTheme();
  themeSwitch.addEventListener("click", (e) => {
    const b = e.target.closest("button[data-choice]");
    if (b) {
      chooseTheme(b.dataset.choice);
    }
  });
  themeSwitch.addEventListener("keydown", (e) => {
    const by = steps.get(e.key);
    if (!by) {
      return;
    }
    e.preventDefault();
    const buttons = Array.from(themeSwitch.querySelectorAll("button"));
    const at = buttons.indexOf(document.activeElement);
    const next = buttons[(at + by + buttons.length) % buttons.length];
    next.focus();
    chooseTheme(next.dataset.choice);
  });
})();
