local telescope = require('telescope')
local builtin = require('telescope.builtin')
local actions = require('telescope.actions')

-- File patterns to ignore
local ignore_patterns = {
  'node_modules',
  '.git/',
  'dist/',
  'build/',
}

-- Layout configuration
local layout_config = {
  horizontal = {
    prompt_position = 'top',
    preview_width = 0.5, -- Exact 50% for symmetry
    results_width = 0.5,
  },
  vertical = {
    mirror = false,
  },
  width = 0.75, -- More compact window
  height = 0.75, -- More compact window
  preview_cutoff = 120,
}

-- Insert mode mappings
local insert_mappings = {
  ['<C-j>'] = actions.move_selection_next,
  ['<C-k>'] = actions.move_selection_previous,
  ['<C-q>'] = actions.send_selected_to_qflist + actions.open_qflist,
  ['<esc>'] = actions.close,
}

-- Main telescope setup
telescope.setup({
  defaults = {
    -- Window transparency
    winblend = 5, -- Very subtle transparency (0=opaque, 100=transparent)
    -- Minimal borders - no visible borders
    borderchars = {
      -- Using spaces for invisible borders
      prompt = { ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ' },
      results = { ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ' },
      preview = { ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ' },
    },
    -- Sort results from top down (ascending)
    sorting_strategy = 'ascending',
    -- Consistent spacing for prompt and selection
    prompt_prefix = ' >  ', -- Space before, two spaces after
    selection_caret = ' >', -- Space before only
    entry_prefix = '  ', -- Two spaces to match the caret width
    mappings = {
      i = insert_mappings,
    },
    file_ignore_patterns = ignore_patterns,
    layout_config = layout_config,
  },
  pickers = {
    find_files = {
      hidden = true,
      find_command = { 'rg', '--files', '--hidden', '--glob', '!.git' },
    },
  },
})

-- Apply custom telescope highlighting
-- Invisible borders - match fg and bg colors
vim.api.nvim_set_hl(0, 'TelescopePromptBorder', { fg = '#3c3836', bg = '#3c3836' })
vim.api.nvim_set_hl(0, 'TelescopeResultsBorder', { fg = '#282828', bg = '#282828' })
vim.api.nvim_set_hl(0, 'TelescopePreviewBorder', { fg = '#1d2021', bg = '#1d2021' })

-- Subtle background differentiation
vim.api.nvim_set_hl(0, 'TelescopePromptNormal', { bg = '#3c3836' })
vim.api.nvim_set_hl(0, 'TelescopeResultsNormal', { bg = '#282828' })
vim.api.nvim_set_hl(0, 'TelescopePreviewNormal', { bg = '#1d2021' })

-- Remove padding between prompt and results
vim.api.nvim_set_hl(0, 'TelescopePromptTitle', { fg = '#ebdbb2', bg = '#3c3836', bold = true })
vim.api.nvim_set_hl(0, 'TelescopeResultsTitle', { fg = '#ebdbb2', bg = '#282828' })
vim.api.nvim_set_hl(0, 'TelescopePreviewTitle', { fg = '#ebdbb2', bg = '#1d2021' })

-- Selection highlighting with color instead of reverse
vim.api.nvim_set_hl(0, 'TelescopeSelection', { bg = '#504945', fg = '#ebdbb2' })
vim.api.nvim_set_hl(0, 'TelescopeSelectionCaret', { bg = '#504945', fg = '#d79921' })

-- NEW KEYBINDING SYSTEM

-- BUFFER OPERATIONS (<leader>b)
vim.keymap.set('n', '<leader>bb', builtin.buffers, { desc = 'Buffer browser' })

-- FILES OPERATIONS (<leader>f)
vim.keymap.set('n', '<leader>fr', builtin.oldfiles, { desc = 'Files recent' })

-- SEARCH OPERATIONS (<leader>s) - CONSOLIDATED SEARCH DOMAIN
vim.keymap.set('n', '<leader>ss', builtin.current_buffer_fuzzy_find, { desc = 'Search picker' })
vim.keymap.set('n', '<leader>sb', builtin.current_buffer_fuzzy_find, { desc = 'Search buffer' })
vim.keymap.set('n', '<leader>sp', builtin.live_grep, { desc = 'Search project' })
vim.keymap.set('n', '<leader>sw', builtin.grep_string, { desc = 'Search word' })
vim.keymap.set('n', '<leader>sy', builtin.lsp_workspace_symbols, { desc = 'Search symbols' })
vim.keymap.set('n', '<leader>sh', builtin.search_history, { desc = 'Search history' })
vim.keymap.set('n', '<leader>sc', function()
  vim.cmd('nohlsearch')
end, { desc = 'Search clear' })
vim.keymap.set('n', '<leader>sH', builtin.help_tags, { desc = 'Search help' })
vim.keymap.set('n', '<leader>sr', function()
  -- Simple search and replace using built-in substitution
  local search_term = vim.fn.input('Search for: ')
  if search_term ~= '' then
    local replace_term = vim.fn.input('Replace with: ')
    local confirm = vim.fn.input('Replace all occurrences? (y/n): ')
    if confirm:lower() == 'y' then
      vim.cmd(string.format('%%s/%s/%s/g', vim.fn.escape(search_term, '/'), vim.fn.escape(replace_term, '/')))
    else
      vim.cmd(string.format('%%s/%s/%s/gc', vim.fn.escape(search_term, '/'), vim.fn.escape(replace_term, '/')))
    end
  end
end, { desc = 'Search replace' })

-- GIT OPERATIONS (<leader>gc, <leader>gb)
vim.keymap.set('n', '<leader>gcv', builtin.git_commits, { desc = 'Git commit view' })
vim.keymap.set('n', '<leader>gcf', builtin.git_bcommits, { desc = 'Git commit file' })
vim.keymap.set('n', '<leader>gbl', builtin.git_branches, { desc = 'Git branch list' })

-- INTROSPECTION (<leader>i) - NEW DOMAIN
vim.keymap.set('n', '<leader>ik', builtin.keymaps, { desc = 'Introspect keymaps' })
vim.keymap.set('n', '<leader>ic', builtin.commands, { desc = 'Introspect commands' })
vim.keymap.set('n', '<leader>io', builtin.vim_options, { desc = 'Introspect options' })
vim.keymap.set('n', '<leader>ih', builtin.command_history, { desc = 'Introspect history' })
vim.keymap.set('n', '<leader>is', builtin.search_history, { desc = 'Introspect search' })
vim.keymap.set('n', '<leader>ir', builtin.registers, { desc = 'Introspect registers' })
vim.keymap.set('n', '<leader>im', builtin.marks, { desc = 'Introspect marks' })
vim.keymap.set('n', '<leader>iH', builtin.help_tags, { desc = 'Introspect Help' })
vim.keymap.set('n', '<leader>il', function()
  -- LSP server status introspection
  vim.cmd('LspInfo')
end, { desc = 'Introspect LSP' })
vim.keymap.set('n', '<leader>it', function()
  -- Treesitter parser information
  vim.cmd('TSPlayground')
end, { desc = 'Introspect treesitter' })

-- UI SETTINGS (<leader>u) - NEW DOMAIN
vim.keymap.set('n', '<leader>uc', builtin.colorscheme, { desc = 'UI colorscheme' })

-- SPECIAL ACCESS
vim.keymap.set('n', '<leader><leader>', builtin.resume, { desc = 'Resume last' })
vim.keymap.set('n', '<leader>/', builtin.current_buffer_fuzzy_find, { desc = 'Quick search' })
