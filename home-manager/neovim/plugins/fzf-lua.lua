local ok, fzf = pcall(require, 'fzf-lua')
if not ok then
  vim.notify('Failed to load fzf-lua', vim.log.levels.ERROR)
  return
end

-- ============================================================================
-- FZF-LUA SETUP
-- ============================================================================

fzf.setup({
  winopts = {
    preview = {
      scrollbar = 'float', -- float looks better than border
    },
  },
  files = {
    rg_opts = [[ --color=never --files --hidden --follow -g '!.git']],
  },
  live_grep = {
    multiprocess = true,
  },
})

-- ============================================================================
-- FILES DOMAIN - Enhanced file operations
-- ============================================================================

vim.keymap.set('n', '<leader>ff', fzf.files, { desc = 'Find files' })
vim.keymap.set('n', '<leader>fo', function()
  fzf.files({ cwd_only = false })
end, { desc = 'Files open (global)' })
vim.keymap.set('n', '<leader>fr', fzf.oldfiles, { desc = 'Find recent files' })
vim.keymap.set('n', '<leader>fg', fzf.git_files, { desc = 'Find git files' })

-- ============================================================================
-- SEARCH DOMAIN - Enhanced search operations
-- ============================================================================

vim.keymap.set('n', '<leader><leader>', fzf.global, { desc = 'Global picker (VS Code-like)' })
vim.keymap.set('n', '<leader>sb', fzf.blines, { desc = 'Search buffer' })
vim.keymap.set('n', '<leader>sB', fzf.lines, { desc = 'Search all buffers' })
vim.keymap.set('n', '<leader>sp', fzf.live_grep, { desc = 'Search project' })
vim.keymap.set('n', '<leader>sw', fzf.grep_cword, { desc = 'Search word' })
vim.keymap.set('n', '<leader>sy', fzf.lsp_workspace_symbols, { desc = 'Search symbols' })
vim.keymap.set('n', '<leader>sh', fzf.search_history, { desc = 'Search history' })
vim.keymap.set('n', '<leader>sH', fzf.helptags, { desc = 'Search help' })
vim.keymap.set('n', '<leader>sq', fzf.quickfix, { desc = 'Search quickfix' })
vim.keymap.set('n', '<leader>sQ', fzf.loclist, { desc = 'Search location list' })
vim.keymap.set('n', '<leader>st', fzf.treesitter, { desc = 'Search treesitter symbols' })

-- Visual mode search
vim.keymap.set('v', '<leader>sv', fzf.grep_visual, { desc = 'Search visual selection' })

-- ============================================================================
-- BUFFER DOMAIN - Enhanced buffer management
-- ============================================================================

vim.keymap.set('n', '<leader>bb', fzf.buffers, { desc = 'Browse buffers' })

-- ============================================================================
-- GIT DOMAIN - Enhanced git workflow with reorganized structure
-- ============================================================================

-- Git Status (primary interface)
vim.keymap.set('n', '<leader>gg', fzf.git_status, { desc = 'Git status' })

-- Archive/Stash operations (<leader>ga*)
vim.keymap.set('n', '<leader>gaa', fzf.git_stash, { desc = 'Archive stash (list)' })

-- Branch operations (<leader>gb*)
vim.keymap.set('n', '<leader>gbb', fzf.git_branches, { desc = 'Browse branches' })

-- Commit operations (<leader>gc*)
vim.keymap.set('n', '<leader>gcc', fzf.git_commits, { desc = 'Commit commits (list)' })
vim.keymap.set('n', '<leader>gcf', fzf.git_bcommits, { desc = 'Commit file history' })

-- Tag operations (<leader>gt*)
vim.keymap.set('n', '<leader>gtt', fzf.git_tags, { desc = 'Tag tags (list)' })

-- Working directory operations (<leader>gw*)
vim.keymap.set('n', '<leader>gww', fzf.git_hunks, { desc = 'Working hunks (list)' })
vim.keymap.set('n', '<leader>gwb', fzf.git_blame, { desc = 'Working blame' })

-- Additional git operations
vim.keymap.set('n', '<leader>gd', function()
  fzf.git_diff({ ref = 'HEAD' })
end, { desc = 'Git diff files' })

-- ============================================================================
-- LSP DOMAIN - Enhanced LSP experience
-- ============================================================================

-- Global LSP keybindings (always available)
vim.keymap.set('n', '<leader>cs', fzf.lsp_document_symbols, { desc = 'Code symbols (document)' })
vim.keymap.set('n', '<leader>cf', fzf.lsp_finder, { desc = 'Code finder (LSP combined)' })

-- LSP keybindings that require LSP attachment
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('FzfLuaLspConfig', {}),
  callback = function(ev)
    local ok, fzf = pcall(require, 'fzf-lua')
    if not ok then
      return
    end

    local bufnr = ev.buf
    local opts = { buffer = bufnr }

    -- GO-TO NAVIGATION (g prefix) - Enhanced with fzf-lua pickers for better selection
    vim.keymap.set('n', 'gd', fzf.lsp_definitions, vim.tbl_extend('force', opts, { desc = 'Go to definition' }))
    vim.keymap.set('n', 'gD', fzf.lsp_declarations, vim.tbl_extend('force', opts, { desc = 'Go to declaration' }))
    vim.keymap.set(
      'n',
      'gi',
      fzf.lsp_implementations,
      vim.tbl_extend('force', opts, { desc = 'Go to implementation' })
    )
    vim.keymap.set(
      'n',
      'gy',
      fzf.lsp_type_definitions,
      vim.tbl_extend('force', opts, { desc = 'Go to type definition' })
    )
    vim.keymap.set('n', 'gr', fzf.lsp_references, vim.tbl_extend('force', opts, { desc = 'Go to references' }))

    -- CODE ACTIONS - Enhanced with fzf-lua
    vim.keymap.set('n', '<leader>ca', fzf.lsp_code_actions, vim.tbl_extend('force', opts, { desc = 'Code action' }))
    vim.keymap.set('v', '<leader>ca', fzf.lsp_code_actions, vim.tbl_extend('force', opts, { desc = 'Code action' }))

    -- Code calls (sub-domain) - Enhanced with fzf-lua
    vim.keymap.set(
      'n',
      '<leader>cci',
      fzf.lsp_incoming_calls,
      vim.tbl_extend('force', opts, { desc = 'Code calls incoming' })
    )
    vim.keymap.set(
      'n',
      '<leader>cco',
      fzf.lsp_outgoing_calls,
      vim.tbl_extend('force', opts, { desc = 'Code calls outgoing' })
    )
  end,
})

-- ============================================================================
-- INTROSPECTION DOMAIN - Enhanced vim state browsing
-- ============================================================================

vim.keymap.set('n', '<leader>ik', fzf.keymaps, { desc = 'Introspect keymaps' })
vim.keymap.set('n', '<leader>ic', fzf.commands, { desc = 'Introspect commands' })
vim.keymap.set('n', '<leader>io', fzf.nvim_options, { desc = 'Introspect options' })
vim.keymap.set('n', '<leader>ih', fzf.command_history, { desc = 'Introspect command history' })
vim.keymap.set('n', '<leader>ir', fzf.registers, { desc = 'Introspect registers' })
vim.keymap.set('n', '<leader>im', fzf.marks, { desc = 'Introspect marks' })
vim.keymap.set('n', '<leader>ij', fzf.jumps, { desc = 'Introspect jumps' })
vim.keymap.set('n', '<leader>iC', fzf.changes, { desc = 'Introspect changes' })
vim.keymap.set('n', '<leader>ia', fzf.autocmds, { desc = 'Introspect autocommands' })
vim.keymap.set('n', '<leader>ix', fzf.spell_suggest, { desc = 'Introspect spelling suggestions' })

-- ============================================================================
-- UI OPERATIONS - Enhanced interface controls
-- ============================================================================

vim.keymap.set('n', '<leader>uc', fzf.colorschemes, { desc = 'UI colorschemes' })

-- ============================================================================
-- QUICK ACCESS FUNCTIONS
-- ============================================================================

vim.keymap.set('n', '<leader>.', fzf.resume, { desc = 'Resume last picker' })
vim.keymap.set('n', '<leader>/', fzf.blines, { desc = 'Quick buffer search' })

-- ============================================================================
-- COMPLETION FUNCTIONS - Enhanced insert mode completion
-- ============================================================================

vim.keymap.set({ 'n', 'v', 'i' }, '<C-x><C-f>', function()
  fzf.complete_file({ winopts = { preview = { hidden = true } } })
end, { desc = 'Complete file' })

vim.keymap.set({ 'n', 'v', 'i' }, '<C-x><C-p>', function()
  fzf.complete_path()
end, { desc = 'Complete path' })

vim.keymap.set({ 'n', 'v', 'i' }, '<C-x><C-l>', function()
  fzf.complete_line()
end, { desc = 'Complete line' })

