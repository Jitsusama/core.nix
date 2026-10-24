-- Runs the probe vim.g.probe names once Neovim has entered, which a person
-- meets it after, rather than as a -c command, which runs before. A probe
-- that fails says why and quits, so its check fails rather than waits.
vim.api.nvim_create_autocmd('VimEnter', {
  once = true,
  callback = function()
    local ok, problem = xpcall(dofile, debug.traceback, vim.g.probe)
    if not ok then
      io.stdout:write(problem)
      vim.cmd('qa!')
    end
  end,
})
