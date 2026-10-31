-- For neovim-says-the-file-it-has-open: once the terminal has attached, and
-- showing.lua has said the file Neovim started with, goes to another file,
-- then to help, which isn't one, and leaves.
vim.api.nvim_create_autocmd('UIEnter', {
  once = true,
  callback = function()
    vim.defer_fn(function()
      vim.cmd('edit two.txt')
      vim.cmd('help')
      vim.defer_fn(function()
        vim.cmd('qa!')
      end, 200)
    end, 300)
  end,
})
