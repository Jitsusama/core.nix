local wezterm = require("wezterm")
local config = wezterm.config_builder()

-- Font settings
config.font = wezterm.font("MonaspiceXe Nerd Font")
config.font_size = 14

-- Window settings
config.initial_rows = 40
config.initial_cols = 180
config.window_padding = {
  left = 16,
  right = 16,
  top = 12,
  bottom = 12,
}
config.window_close_confirmation = "NeverPrompt"
config.quit_when_all_windows_are_closed = true

-- Background: dark green base (#000A00) with tiled image overlay
config.background = {
  {
    source = { Color = "#000A00" },
    width = "100%",
    height = "100%",
  },
  {
    source = { File = "WALLPAPER_PATH" },
    repeat_x = "Mirror",
    repeat_y = "Mirror",
    opacity = 0.23,
    hsb = { brightness = 1.0 },
  },
}

-- macOS specific
config.send_composed_key_when_left_alt_is_pressed = false
config.send_composed_key_when_right_alt_is_pressed = false
config.window_decorations = "RESIZE"
config.macos_window_background_blur = 0

-- Inactive pane dimming
config.inactive_pane_hsb = {
  brightness = 0.77,
}

-- Cursor
config.default_cursor_style = "SteadyBlock"

-- Scrollback
config.scrollback_lines = 10000

-- Tab title formatting: add padding around tab titles
wezterm.on("format-tab-title", function(tab)
  local title = tab.active_pane.title
  return " " .. title .. " "
end)

-- Tab bar: only show when more than one tab is open
config.hide_tab_bar_if_only_one_tab = true
config.use_fancy_tab_bar = true
config.tab_bar_at_bottom = true
config.tab_max_width = 32

config.window_frame = {
  font = wezterm.font("MonaspiceXe Nerd Font"),
  font_size = 12,
  active_titlebar_bg = "rgba(0, 10, 0, 0.4)",
  inactive_titlebar_bg = "rgba(0, 10, 0, 0.4)",
}

config.colors = {
  tab_bar = {
    active_tab = {
      bg_color = "rgba(10, 31, 10, 0.5)",
      fg_color = "#6A8A6A",
      intensity = "Bold",
    },
    inactive_tab = {
      bg_color = "rgba(0, 10, 0, 0.3)",
      fg_color = "#3A4A3A",
    },
    inactive_tab_hover = {
      bg_color = "rgba(6, 18, 6, 0.5)",
      fg_color = "#5A7A5A",
    },
    new_tab = {
      bg_color = "rgba(0, 10, 0, 0.3)",
      fg_color = "#3A4A3A",
    },
    new_tab_hover = {
      bg_color = "rgba(6, 18, 6, 0.5)",
      fg_color = "#5A7A5A",
    },
  },
}

-- Hyperlink rules: override defaults to properly handle URLs in parens
config.hyperlink_rules = {
  -- Matches: a URL in parens: (URL)
  {
    regex = "\\((\\w+://\\S+)\\)",
    format = "$1",
    highlight = 1,
  },
  -- Matches: a URL in brackets: [URL]
  {
    regex = "\\[(\\w+://\\S+)\\]",
    format = "$1",
    highlight = 1,
  },
  -- Matches: a URL in curly braces: {URL}
  {
    regex = "\\{(\\w+://\\S+)\\}",
    format = "$1",
    highlight = 1,
  },
  -- Matches: a URL in angle brackets: <URL>
  {
    regex = "<(\\w+://\\S+)>",
    format = "$1",
    highlight = 1,
  },
  -- Then handle URLs not wrapped in brackets
  {
    regex = "\\b\\w+://\\S+[/a-zA-Z0-9-]+",
    format = "$0",
  },
  -- implicit mailto link
  {
    regex = "\\b\\w+@[\\w-]+(\\.[\\w-]+)+\\b",
    format = "mailto:$0",
  },
}

return config
