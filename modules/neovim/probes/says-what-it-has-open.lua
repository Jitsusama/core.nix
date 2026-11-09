-- For neovim-says-what-it-has-open: once the terminal has attached, and
-- showing.lua has said where Neovim's server listens, opens the second
-- file in a tab of its own, so the check can ask what's open there, and
-- leaves once the check says it has asked, as a file named asked.
vim.api.nvim_create_autocmd('UIEnter', {
  once = true,
  callback = function()
    vim.cmd('tabedit two.txt')
    local timer = assert(vim.uv.new_timer())
    timer:start(
      100,
      100,
      vim.schedule_wrap(function()
        if vim.uv.fs_stat('asked') then
          timer:stop()
          vim.cmd('qa!')
        end
      end)
    )
  end,
})
