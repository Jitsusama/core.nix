local later = require('later')
local keys = require('keys')
local wrong = {}
local function say(line)
  table.insert(wrong, line)
end
for name in pairs(require('waiting')) do
  if later.loaded(name) then
    say(name .. ' loaded as Neovim started')
  end
end
-- What draws the first screen, and what those need.
local first = {
  ['async.nvim'] = true,
  ['nvim-numbertoggle'] = true,
  ['nvim-treesitter'] = true,
  ['nvim-treesitter-grammars'] = true,
  ['nvim-treesitter-textobjects'] = true,
  ['oil.nvim'] = true,
  ['snacks.nvim'] = true,
  ['vim-nix'] = true,
  ['vim-repeat'] = true,
}
for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/start/*', false, true)) do
  if not first[vim.fs.basename(dir)] then
    say(vim.fs.basename(dir) .. ' starts with Neovim, though the first screen needs none of it')
  end
end
if package.loaded['vim.diagnostic'] then
  say('vim.diagnostic loaded as Neovim started')
end
for _, key in ipairs(keys.keys) do
  local command = key.command and key.command:match('^%S+')
  if command and vim.fn.exists(':' .. command) ~= 2 then
    say(key.lhs .. ' runs ' .. command .. ', which is not there')
  end
end
local windows = #vim.api.nvim_list_wins()
vim.cmd('UndotreeShow')
if not later.loaded('undotree') or #vim.api.nvim_list_wins() == windows then
  say('UndotreeShow did not load undotree and show it')
end
require('trouble')
if not later.loaded('trouble.nvim') then
  say('requiring trouble did not load it')
end
-- aerial's setup requires aerial itself, so the first require has to
-- answer with the copy that was set up; another copy has no settings
-- and fails when asked for an outline. This file has no symbols, so
-- the outline it gives is empty.
local opened, problem = pcall(function()
  require('aerial').nav_open()
end)
if not opened then
  say('aerial, called before it loaded, was not set up: ' .. tostring(problem))
end
later.provide('jump.line')
if not later.loaded('flash.nvim') or keys.actions['jump.line'] == nil then
  say('asking for jump.line did not load flash')
end
vim.api.nvim_exec_autocmds('UIEnter', {})
local background = { 'nvim-lspconfig', 'nvim-cmp', 'gitsigns.nvim', 'which-key.nvim' }
vim.wait(5000, function()
  return vim.iter(background):all(later.loaded)
end)
for _, name in ipairs(background) do
  if not later.loaded(name) then
    say(name .. ' did not load once the first screen was drawn')
  end
end
-- Completion's sources register in their after/plugin files.
vim.wait(2000, function()
  return #require('cmp').core:get_sources() >= 2
end)
local sources = {}
for _, source in ipairs(require('cmp').core:get_sources()) do
  sources[source.name] = true
end
if not (sources.buffer and sources.path) then
  say('suggestions come from ' .. table.concat(vim.tbl_keys(sources), ' '))
end
local library = vim.lsp.config.lua_ls.settings.Lua.workspace.library
if not vim.deep_equal(library, { vim.env.VIMRUNTIME, '${3rd}/luv/library' }) then
  say('lua_ls reads ' .. #library .. ' folders')
end
io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
vim.cmd('qa!')
