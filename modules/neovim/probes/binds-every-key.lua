local keys = require('keys')
local later = require('later')
local wrong = {}
local function say(line)
  table.insert(wrong, line)
end
for _, name in ipairs(keys.missing()) do
  if not later.provides(name) then
    say('nothing provides ' .. name)
  end
end
later.load_all()
for _, name in ipairs(keys.missing()) do
  say('no action ' .. name)
end
-- Every scope arrives in this one buffer, as each would in its own.
for _, scope in ipairs({ 'code', 'git', 'status' }) do
  keys.attach(scope, 0)
end
local ours = {}
for _, key in
  ipairs(vim.tbl_filter(function(key)
    return key.plugin == nil
  end, keys.keys))
do
  for _, mode in ipairs(key.modes) do
    local map = vim.fn.maparg(key.lhs, mode, false, true)
    if map.desc ~= key.desc then
      say(mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
    elseif (map.buffer == 1) ~= (key.scope ~= nil) then
      say(mode .. ' ' .. key.lhs .. ' is in the wrong place')
    else
      ours[mode .. map.lhsraw] = true
    end
  end
end
for _, mode in ipairs({ 'n', 'x', 'o', 'i', 't' }) do
  local maps = vim.list_extend(vim.api.nvim_get_keymap(mode), vim.api.nvim_buf_get_keymap(0, mode))
  for _, map in ipairs(maps) do
    local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
    -- A key set once Neovim had started has no script, nor was it
    -- set by the configuration.
    local from = map.sid > 0 and (vim.fn.getscriptinfo({ sid = map.sid })[1] or {}).name or ''
    if map.desc and not ours[mode .. raw] and from:find('/config/nvim/', 1, true) then
      say('stray ' .. mode .. ' ' .. map.lhs .. ' ' .. map.desc)
    end
  end
end

-- A plugin's window binds its own keys, so each window is opened
-- as it would be, and holds its keys from keys.nix and no others
-- but a mouse's.
vim.fn.writefile({ 'local function hello()', 'end' }, 'probe.lua')
vim.cmd('edit probe.lua')
local function window(scope, filetype, open, theirs)
  open()
  if not vim.wait(5000, function()
    return vim.bo.filetype == filetype
  end) then
    say('no ' .. scope .. ' window')
    return
  end
  local mine, modes = {}, {}
  for _, key in ipairs(keys.keys) do
    if key.scope == scope then
      for _, mode in ipairs(key.modes) do
        modes[mode] = true
        local map = vim.fn.maparg(key.lhs, mode, false, true)
        if map.desc ~= key.desc or map.buffer ~= 1 then
          say(scope .. ' ' .. mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
        else
          mine[mode .. map.lhsraw] = true
        end
      end
    end
  end
  for mode in pairs(modes) do
    for _, map in ipairs(vim.api.nvim_buf_get_keymap(0, mode)) do
      local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
      if not mine[mode .. raw] and not map.lhs:find('Mouse') and not (theirs and theirs(map)) then
        say('stray ' .. scope .. ' ' .. mode .. ' ' .. map.lhs .. ' ' .. tostring(map.desc))
      end
    end
  end
end
window('outline', 'aerial', function()
  require('aerial').open({ focus = true })
end)
vim.cmd('AerialClose')
window('walk', 'aerial-nav', function()
  require('aerial').nav_open()
end)
vim.wait(200)
require('aerial').nav_close()
vim.wait(200)
window('files', 'oil', function()
  vim.cmd('Oil .')
end)
vim.cmd('edit probe.lua')
window('problems', 'trouble', function()
  vim.fn.setqflist({ { filename = 'probe.lua', lnum = 1, text = 'a problem' } })
  require('trouble').toggle('quickfix')
end)
-- octo binds a buffer's keys as it opens one, through apply_mappings,
-- and its notifications' in the finder, from its configuration.
local octo = {
  ['octo-issue'] = 'issue',
  ['octo-pull'] = 'pull_request',
  ['octo-thread'] = 'review_thread',
  ['octo-submit'] = 'submit_win',
  ['octo-change'] = 'review_diff',
  ['octo-files'] = 'file_panel',
}
for _, scope in ipairs(vim.tbl_keys(octo)) do
  -- autopairs binds its own keys for typing in every buffer.
  window(scope, 'octo-probe', function()
    vim.cmd('enew')
    vim.bo.filetype = 'octo-probe'
    require('octo.utils').apply_mappings(octo[scope], 0)
  end, function(map)
    return (map.desc or ''):find('^autopairs') ~= nil
  end)
end
-- Its notifications' and workflow runs' keys octo binds its own
-- way, so they're read from its configuration, and its runs'
-- view, which reads them as it loads, loads.
local given = require('octo.config').values.mappings
local theirs = { ['octo-notices'] = 'notification', ['octo-runs'] = 'runs' }
for _, key in ipairs(keys.keys) do
  if theirs[key.scope] then
    local action = (given[theirs[key.scope]] or {})[key.plugin] or {}
    if action.lhs ~= key.lhs or action.desc ~= key.desc then
      say(key.scope .. ' ' .. key.lhs .. ' is not ' .. key.plugin)
    end
  end
end
local loaded, problem = pcall(require, 'octo.workflow_runs')
if not loaded then
  say('octo cannot show a workflow run: ' .. tostring(problem))
end
-- cmp binds the suggestions' keys as it suggests, from these.
local mapping = require('cmp').get_config().mapping
local suggesting = vim.tbl_filter(function(key)
  return key.scope == 'completion'
end, keys.keys)
for _, key in ipairs(suggesting) do
  if mapping[require('cmp.utils.keymap').normalize(key.lhs)] == nil then
    say('completion ' .. key.lhs .. ' is unbound')
  end
end
if vim.tbl_count(mapping) ~= #suggesting then
  say('completion has ' .. vim.tbl_count(mapping) .. ' keys, not ' .. #suggesting)
end
-- The command line's are its own, for a command and a search alike.
local lines = require('cmp.config').cmdline
local commanding = vim.tbl_filter(function(key)
  return key.scope == 'commands'
end, keys.keys)
for _, kind in ipairs({ ':', '/', '?' }) do
  local line = require('cmp.config').normalize(lines[kind] or {}).mapping or {}
  for _, key in ipairs(commanding) do
    local bound = line[require('cmp.utils.keymap').normalize(key.lhs)]
    if bound == nil or bound.c == nil then
      say('command line ' .. kind .. ' ' .. key.lhs .. ' is unbound')
    end
  end
  if vim.tbl_count(line) ~= #commanding then
    say('command line ' .. kind .. ' has ' .. vim.tbl_count(line) .. ' keys, not ' .. #commanding)
  end
end
io.stdout:write(#wrong == 0 and 'every key' or table.concat(wrong, '\n'))
