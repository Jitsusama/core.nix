-- Twilight: Dims inactive portions of code for better focus

local theme = require('theme')

require('twilight').setup({
  dimming = {
    -- The colour scheme draws no background, so twilight dims the text
    -- towards the terminal's.
    color = { 'Normal', theme.text },
    term_bg = theme.background,
  },
  expand = {
    'function',
    'method',
    'table',
    'if_statement',
    'for_statement',
    'while_statement',
    'class',
    'module',
    'block',
  },
  exclude = {
    'oil',
    'notify',
  },
})
