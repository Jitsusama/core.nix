local gitsigns = require('gitsigns')

gitsigns.setup({
  signs = {
    add          = { text = '┃' },
    change       = { text = '┃' },
    delete       = { text = '_' },
    topdelete    = { text = '‾' },
    changedelete = { text = '~' },
    untracked    = { text = '┆' },
  },
  signcolumn = true,
  numhl = false,
  linehl = false,
  word_diff = false,
  watch_gitdir = {
    interval = 1000,
    follow_files = true
  },
  attach_to_untracked = true,
  current_line_blame = false,
  current_line_blame_opts = {
    virt_text = true,
    virt_text_pos = 'eol',
    delay = 1000,
    ignore_whitespace = false,
  },
  sign_priority = 6,
  update_debounce = 100,
  status_formatter = nil,
  max_file_length = 40000,
  preview_config = {
    border = 'single',
    style = 'minimal',
    relative = 'cursor',
    row = 0,
    col = 1
  },
  on_attach = function(bufnr)
    local function map(mode, l, r, opts)
      opts = opts or {}
      opts.buffer = bufnr
      vim.keymap.set(mode, l, r, opts)
    end

    -- Navigation
    map('n', ']g', function()
      if vim.wo.diff then
        vim.cmd.normal({']c', bang = true})
      else
        gitsigns.nav_hunk('next')
      end
    end, {desc = 'Next git change'})

    map('n', '[g', function()
      if vim.wo.diff then
        vim.cmd.normal({'[c', bang = true})
      else
        gitsigns.nav_hunk('prev')
      end
    end, {desc = 'Previous git change'})

    -- WORKING operations (w prefix) - manipulate working directory
    map('n', '<leader>gwh', gitsigns.preview_hunk_inline, {desc = 'Git working hunk preview'})
    map('n', '<leader>gwv', gitsigns.diffthis, {desc = 'Git working view diff'})
    map('n', '<leader>gwH', function() gitsigns.diffthis('~') end, {desc = 'Git working vs HEAD diff'})
    map('n', '<leader>gwd', gitsigns.reset_hunk, {desc = 'Git working discard hunk'})
    map('v', '<leader>gwd', function() gitsigns.reset_hunk {vim.fn.line('.'), vim.fn.line('v')} end, {desc = 'Git working discard selection'})
    map('n', '<leader>gwD', gitsigns.reset_buffer, {desc = 'Git working Discard file'})

    -- STAGE operations (s prefix) - move in/out of staging
    map('n', '<leader>gsa', gitsigns.stage_hunk, {desc = 'Git stage add hunk'})
    map('v', '<leader>gsa', function() gitsigns.stage_hunk {vim.fn.line('.'), vim.fn.line('v')} end, {desc = 'Git stage add selection'})
    map('n', '<leader>gsA', gitsigns.stage_buffer, {desc = 'Git stage Add file'})
    map('n', '<leader>gsu', function() gitsigns.reset_hunk({staged = true}) end, {desc = 'Git stage undo hunk'})
    map('n', '<leader>gsv', function() gitsigns.diffthis('--staged') end, {desc = 'Git stage view'})

    -- COMMIT operations (c prefix) - commit-related viewing
    map('n', '<leader>gcb', function() gitsigns.blame_line{full=true} end, {desc = 'Git commit blame'})
    map('n', '<leader>gctb', gitsigns.toggle_current_line_blame, {desc = 'Git commit toggle blame'})

    -- Text object
    map({'o', 'x'}, 'ih', ':<C-U>Gitsigns select_hunk<CR>', {desc = 'Select hunk'})
  end
})

