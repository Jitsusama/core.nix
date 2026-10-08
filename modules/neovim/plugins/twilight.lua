-- Twilight: Dims inactive portions of code for better focus

require('twilight').setup({
  dimming = {
    -- Customize for gruvbox theme
    color = { 'Normal', '#ebdbb2' },
    term_bg = '#282828', -- Gruvbox dark background
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

-- Keybinding following UI domain pattern
vim.keymap.set('n', '<leader>ut', '<cmd>Twilight<cr>', { desc = 'UI twilight (focus mode)' })

