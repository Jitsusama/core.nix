-- Neovim's keys, from the list home.nix writes from keys.nix, which says
-- what each is for.
local M = { actions = {}, keys = require('bindings') }

-- A function for a key to run, by the name keys.nix gives it.
function M.act(name, fn)
  M.actions[name] = fn
end

local function rhs(key)
  if key.lua then
    return key.lua
  elseif key.action then
    return function(...)
      local fn = M.actions[key.action]
      if fn == nil and require('later').provide(key.action) then
        fn = M.actions[key.action]
      end
      if fn == nil then
        vim.notify('Nothing does ' .. key.action .. ' yet', vim.log.levels.ERROR)
        return
      end
      return fn(...)
    end
  elseif key.command then
    return '<cmd>' .. key.command .. '<cr>'
  end
  return key.keys
end

local function bind(key, buffer)
  vim.keymap.set(key.modes, key.lhs, rhs(key), {
    desc = key.desc,
    buffer = buffer,
    expr = key.expr,
    remap = key.remap,
    silent = key.command ~= nil,
  })
end

-- A scope's keys, in the buffer it has arrived in.
function M.attach(scope, buffer)
  for _, key in ipairs(M.keys) do
    if key.scope == scope and key.plugin == nil then
      bind(key, buffer)
    end
  end
end

-- A plugin window's keys, for its plugin to bind: each key's spelling,
-- to what make turns it into in the plugin's own terms.
function M.window(scope, make)
  local window = {}
  for _, key in ipairs(M.keys) do
    if key.scope == scope and key.plugin ~= nil then
      window[key.lhs] = make(key)
    end
  end
  return window
end

-- A key run as if pressed: a plugin's own by pressing it, since its
-- plugin bound it, and the rest by what keys.nix says it does.
local function run(key)
  if key.plugin then
    return vim.api.nvim_feedkeys(vim.keycode(key.lhs), 'm', false)
  end
  local how = rhs(key)
  if type(how) == 'function' then
    return how()
  end
  vim.api.nvim_feedkeys(vim.keycode(how), key.remap and 'm' or 'n', false)
end

-- Whether a key works here: everywhere, or bound in this buffer as this
-- key, since a plugin's windows can give one chord different keys.
local function here(key)
  if key.scope == nil then
    return true
  end
  local bound = vim.fn.maparg(key.lhs, 'n', false, true)
  return bound.buffer == 1 and bound.desc == key.desc
end

-- Words in one of the theme's roles, as the finder draws them.
local function painted(role, words)
  local hex = require('theme')[role]
  local red, green, blue =
    tonumber(hex:sub(2, 3), 16), tonumber(hex:sub(4, 5), 16), tonumber(hex:sub(6, 7), 16)
  return ('\27[38;2;%d;%d;%dm%s\27[0m'):format(red, green, blue, words)
end

-- A group's keys, found by what they do and run as if pressed: Space
-- in a group whose keys are things to do rather than things to find.
-- Each row reads as the launcher's do, what it does and then its keys,
-- one step quieter, in a column of their own.
function M.find(group, name)
  local found, seen, widest = {}, {}, 0
  for _, key in ipairs(M.keys) do
    local rest = key.chord:sub(#group + 2)
    if
      vim.tbl_contains(key.modes, 'n')
      and not seen[key.lhs]
      and here(key)
      and key.chord:sub(1, #group + 1) == group .. ' '
      and rest ~= 'Space'
    then
      table.insert(found, { key = key, rest = rest })
      seen[key.lhs] = true
      widest = math.max(widest, vim.fn.strdisplaywidth(key.desc))
    end
  end
  if #found == 0 then
    vim.notify('No key for ' .. name .. ' works here', vim.log.levels.INFO)
    return
  end
  vim.ui.select(found, {
    prompt = (name:gsub('^%l', string.upper)),
    format_item = function(item)
      local gap = string.rep(' ', widest - vim.fn.strdisplaywidth(item.key.desc) + 2)
      return painted('strong', item.key.desc) .. gap .. painted('text', item.rest)
    end,
  }, function(item)
    if item then
      run(item.key)
    end
  end)
end

-- The names keys.nix gives that no file has registered.
function M.missing()
  local names = {}
  for _, key in ipairs(M.keys) do
    if key.action and M.actions[key.action] == nil then
      table.insert(names, key.action)
    end
  end
  return names
end

for _, key in ipairs(M.keys) do
  if key.scope == nil then
    bind(key)
  end
end

return M
