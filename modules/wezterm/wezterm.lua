local wezterm = require('wezterm')
-- The font, colours and padding, written by home.nix from jitsusama.theme.
local theme = require('theme')
local config = wezterm.config_builder()

config.font = wezterm.font(theme.font)
config.font_size = theme.font_size
config.colors = theme.colors

-- Window settings
config.initial_rows = 40
config.initial_cols = 180
config.window_padding = theme.padding
config.window_close_confirmation = 'NeverPrompt'
config.quit_when_all_windows_are_closed = true

-- Background: the theme's, with a tiled image over it
config.background = {
  {
    source = { Color = theme.colors.background },
    width = '100%',
    height = '100%',
  },
  {
    source = { File = 'WALLPAPER_PATH' },
    repeat_x = 'Mirror',
    repeat_y = 'Mirror',
    opacity = 0.23,
    hsb = { brightness = 1.0 },
  },
}

-- macOS specific
config.send_composed_key_when_left_alt_is_pressed = false
config.send_composed_key_when_right_alt_is_pressed = false
config.window_decorations = 'RESIZE'
config.macos_window_background_blur = 0

-- Inactive pane dimming
config.inactive_pane_hsb = {
  brightness = 0.77,
}

-- Cursor
config.default_cursor_style = 'SteadyBlock'

-- Keyboard
config.enable_kitty_keyboard = true

-- Scrollback
config.scrollback_lines = 10000

-- Tab title formatting: add padding around tab titles
wezterm.on('format-tab-title', function(tab)
  local title = tab.active_pane.title
  return ' ' .. title .. ' '
end)

-- Tab bar: only show when more than one tab is open
config.hide_tab_bar_if_only_one_tab = true
config.use_fancy_tab_bar = true
config.tab_bar_at_bottom = true
config.tab_max_width = 32

config.window_frame = {
  font = wezterm.font(theme.font),
  font_size = theme.font_size,
  active_titlebar_bg = theme.frame.active_titlebar_bg,
  inactive_titlebar_bg = theme.frame.inactive_titlebar_bg,
}

-- Hyperlink rules: override defaults to properly handle URLs in parens
config.hyperlink_rules = {
  -- Matches: a URL in parens: (URL)
  {
    regex = '\\((\\w+://\\S+)\\)',
    format = '$1',
    highlight = 1,
  },
  -- Matches: a URL in brackets: [URL]
  {
    regex = '\\[(\\w+://\\S+)\\]',
    format = '$1',
    highlight = 1,
  },
  -- Matches: a URL in curly braces: {URL}
  {
    regex = '\\{(\\w+://\\S+)\\}',
    format = '$1',
    highlight = 1,
  },
  -- Matches: a URL in angle brackets: <URL>
  {
    regex = '<(\\w+://\\S+)>',
    format = '$1',
    highlight = 1,
  },
  -- Then handle URLs not wrapped in brackets
  {
    regex = '\\b\\w+://\\S+[/a-zA-Z0-9-]+',
    format = '$0',
  },
  -- implicit mailto link
  {
    regex = '\\b\\w+@[\\w-]+(\\.[\\w-]+)+\\b',
    format = 'mailto:$0',
  },
}

return config
