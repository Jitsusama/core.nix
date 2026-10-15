-- Tree-sitter for every buffer whose language has a parser: its highlights,
-- indent and folds, and text objects to select and move by, whose keys are
-- in keys.nix. nvim-treesitter now only installs parsers and queries, and
-- each buffer starts Tree-sitter itself; Neovim selects nodes on its own
-- (an and in in visual mode).

require('nvim-treesitter').setup({})

vim.api.nvim_create_autocmd('FileType', {
  group = vim.api.nvim_create_augroup('treesitter', { clear = true }),
  callback = function(args)
    -- A language with no parser keeps Vim's own syntax.
    if not pcall(vim.treesitter.start, args.buf) then
      return
    end
    vim.bo[args.buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
  end,
})
-- Folds are there to close, not closed to start with.
vim.opt.foldenable = false

require('nvim-treesitter-textobjects').setup({
  select = { lookahead = true },
  move = { set_jumps = true },
})
