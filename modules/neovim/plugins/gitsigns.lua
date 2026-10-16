local gitsigns = require('gitsigns')

gitsigns.setup({
  signs = {
    add = { text = '┃' },
    change = { text = '┃' },
    delete = { text = '_' },
    topdelete = { text = '‾' },
    changedelete = { text = '~' },
    untracked = { text = '┆' },
  },
  signcolumn = true,
  numhl = false,
  linehl = false,
  word_diff = false,
  -- Git's folder is watched by the file system's change events, so
  -- nothing polls.
  watch_gitdir = {
    follow_files = true,
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
    border = 'solid',
    style = 'minimal',
    relative = 'cursor',
    row = 0,
    col = 1,
  },
  -- A file git tracks gets git's keys (keys.nix).
  on_attach = function(bufnr)
    require('keys').attach('git', bufnr)
  end,
})
