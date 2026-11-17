# The one browser a headless Playwright launch opens: Chromium's headless
# shell, which `chromium.launch()` runs whenever headless is left on. Full
# Chromium, Firefox, WebKit and the ffmpeg that records video stay out, as
# nothing here opens a window or records one. `browsers-chromium` would
# not do: it carries full Chromium without the shell.
{ playwright-driver }:
playwright-driver.passthru.selectBrowsers {
  withChromium = false;
  withFirefox = false;
  withWebkit = false;
  withFfmpeg = false;
}
