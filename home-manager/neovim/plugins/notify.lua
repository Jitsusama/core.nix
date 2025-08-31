local notify = require('notify')

local icons = {
  times_circle = vim.fn.nr2char(0xf00d),
  exclamation_triangle = vim.fn.nr2char(0xf071),
  info_circle = vim.fn.nr2char(0xf05a),
  bug = vim.fn.nr2char(0xf188),
  stack_trace = vim.fn.nr2char(0xf24e),
}

-- Configure nvim-notify with gruvbox-compatible styling
notify.setup({
  -- Animation style - subtle fade to match professional workflow
  stages = 'fade_in_slide_out',

  -- Default timeout - 3 seconds for most notifications
  timeout = 3000,

  -- Background color for transparency effects
  background_colour = '#282828', -- Gruvbox dark background

  icons = {
    ERROR = icons.times_circle,
    WARN = icons.exclamation_triangle,
    INFO = icons.info_circle,
    DEBUG = icons.bug,
    TRACE = icons.stack_trace,
  },

  -- Window dimensions
  minimum_width = 50,
  max_width = 80,
  max_height = 20,

  -- Use default renderer
  render = 'default',

  -- Default to INFO level
  level = vim.log.levels.INFO,

  -- Notifications appear from top-right
  top_down = true,
})

-- Set as default notify handler
vim.notify = notify
