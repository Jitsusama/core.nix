-- Plugins that wait until they're wanted. Neovim starts with only what
-- draws its first screen; every other plugin loads the first time
-- something asks for it: one of its modules required, one of its commands
-- run, one of its actions pressed, an event it waits for, or the first
-- screen drawn, for those that work in the background.
--
-- home.nix says how each plugin waits, and waiting.lua, which a build
-- writes by loading every plugin once (M.find), says which modules and
-- commands each one brings.
local M = {}

local waiting = require('waiting')
local specs = {}
local loaded = {}
local providers = {}
local owners = {}
local soon = {}
local problems = 0
-- While a build finds what each plugin brings: what it has found, and the
-- commands already put down to a plugin.
local finding
local group = vim.api.nvim_create_augroup('later', { clear = true })

-- Every module a plugin brings, by its first part, names the plugins that
-- bring something under it. It can be more than one: todo-comments brings
-- trouble.sources.todo, inside trouble.nvim's own trouble.
local function own(module, name)
  owners[module] = owners[module] or {}
  table.insert(owners[module], name)
end
for name, brings in pairs(waiting) do
  for _, module in ipairs(brings.modules) do
    own(module, name)
  end
end
for _, names in pairs(owners) do
  table.sort(names)
end

-- The groups of autocmds an event would run now, so the ones a plugin
-- adds while it loads can be run for the event that woke it.
local function groups(event)
  local found = {}
  for _, autocmd in ipairs(vim.api.nvim_get_autocmds({ event = event })) do
    if autocmd.group then
      found[autocmd.group] = true
    end
  end
  return found
end

local function complain(name, problem)
  problems = problems + 1
  if finding then
    io.stderr:write(name .. ' did not load: ' .. problem .. '\n')
  else
    vim.notify(name .. ' did not load: ' .. problem, vim.log.levels.ERROR)
  end
end

-- The table holding a function by its path, such as vim.ui for
-- vim.ui.select, and the function's name in it.
local function holder(path)
  local parent, key = path:match('^(.*)%.([^.]+)$')
  return vim.tbl_get(_G, unpack(vim.split(parent, '.', { plain = true }))), key
end

-- Runs a plugin's after/plugin files that haven't run, which :packadd
-- never does and Neovim does only for what's there as it starts. Some
-- plugins do their work there: completion's sources register themselves
-- in theirs. A plugin loaded while Neovim starts waits until it has, since
-- Neovim may yet run them itself.
local function run_after(name)
  if vim.v.vim_did_enter == 0 then
    vim.api.nvim_create_autocmd('VimEnter', {
      group = group,
      once = true,
      callback = function()
        run_after(name)
      end,
    })
    return
  end
  local ran = {}
  for _, script in ipairs(vim.fn.getscriptinfo()) do
    ran[vim.fs.normalize(vim.fn.resolve(script.name))] = true
  end
  for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/opt/' .. name, false, true)) do
    local files = vim.fn.globpath(dir, 'after/plugin/**/*.{vim,lua}', false, true)
    table.sort(files)
    for _, file in ipairs(files) do
      if not ran[vim.fs.normalize(vim.fn.resolve(file))] then
        vim.cmd.source(vim.fn.fnameescape(file))
      end
    end
  end
end

local function configure(name, config)
  local ok, problem = xpcall(config, debug.traceback)
  if not ok then
    complain(name, problem)
  end
end

-- Loads a plugin, and before it what it needs and after it what goes
-- with it. Loading one twice does nothing. One asked for before it's
-- been said how it waits loads bare, and its config runs once it's said.
function M.load(name)
  if loaded[name] or (specs[name] == nil and waiting[name] == nil) then
    return
  end
  local spec = specs[name] or { stand_ins = {}, waits = {}, replaced = {} }
  loaded[name] = true
  local before = finding and vim.api.nvim_get_commands({})
  for _, need in ipairs(spec.needs or {}) do
    M.load(need)
  end
  for _, command in ipairs(spec.stand_ins) do
    pcall(vim.api.nvim_del_user_command, command)
  end
  for _, id in ipairs(spec.waits) do
    pcall(vim.api.nvim_del_autocmd, id)
  end
  for path, stand_in in pairs(spec.replaced) do
    local held, key = holder(path)
    if held[key] == stand_in then
      held[key] = spec.originals[path]
    end
  end
  local ok, problem = pcall(vim.cmd.packadd, name)
  if not ok then
    return complain(name, problem)
  end
  ok, problem = pcall(run_after, name)
  if not ok then
    complain(name, problem)
  end
  if spec.config then
    configure(name, spec.config)
  end
  for _, also in ipairs(spec.also or {}) do
    M.load(also)
  end
  -- What it made and the plugins it loaded didn't is its own.
  if finding then
    for command in pairs(vim.api.nvim_get_commands({})) do
      if not before[command] and not finding.claimed[command] then
        finding.claimed[command] = true
        table.insert(finding.brings[name].commands, command)
      end
    end
  end
end

-- Writes what each plugin brings to out as waiting.lua, loading every
-- plugin in turn, and answers how many failed to load. A build runs it
-- with an empty waiting.lua in place.
function M.find(out)
  finding = { brings = {}, claimed = {} }
  for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/opt/*', false, true)) do
    local name = vim.fs.basename(dir)
    local found = {}
    -- Neovim's own optional packs aren't ours to wake.
    if specs[name] and vim.uv.fs_stat(dir .. '/lua') then
      for entry, kind in vim.fs.dir(dir .. '/lua') do
        if kind == 'directory' or kind == 'link' then
          found[entry] = true
        elseif entry:match('%.lua$') then
          found[(entry:gsub('%.lua$', ''))] = true
        end
      end
    end
    if specs[name] then
      local modules = vim.tbl_keys(found)
      table.sort(modules)
      finding.brings[name] = { modules = modules, commands = {} }
      for _, module in ipairs(modules) do
        own(module, name)
      end
    end
  end
  local names = vim.tbl_keys(specs)
  table.sort(names)
  for _, name in ipairs(names) do
    if finding.brings[name] == nil then
      complain(name, 'no such plugin waits')
    else
      M.load(name)
    end
  end
  for _, brings in pairs(finding.brings) do
    table.sort(brings.commands)
  end
  local file = assert(io.open(out, 'w'))
  file:write('return vim.json.decode([==[' .. vim.json.encode(finding.brings) .. ']==])\n')
  file:close()
  return problems
end

-- Loads every plugin, for the checks, which hold each one to what it
-- promises.
function M.load_all()
  for name in pairs(specs) do
    M.load(name)
  end
end

function M.loaded(name)
  return loaded[name] == true
end

-- The plugin that provides an action, if one waiting does.
function M.provides(action)
  return providers[action]
end

-- Loads the plugin an action belongs to, for a key pressed before it has.
function M.provide(action)
  local name = providers[action]
  if name then
    M.load(name)
    return true
  end
  return false
end

-- A command of a plugin not yet loaded stands in for it: run, it loads the
-- plugin and runs the real one with everything it was given, and asked to
-- complete, it loads the plugin and completes as the real one would.
local function stand_in(name, command)
  vim.api.nvim_create_user_command(command, function(event)
    M.load(name)
    local range = ''
    if event.range == 1 then
      range = tostring(event.line1)
    elseif event.range == 2 then
      range = event.line1 .. ',' .. event.line2
    end
    local line = range .. command .. (event.bang and '!' or '') .. ' ' .. event.args
    vim.cmd(event.mods ~= '' and event.mods .. ' ' .. line or line)
  end, {
    nargs = '*',
    bang = true,
    range = true,
    complete = function(_, line)
      M.load(name)
      return vim.fn.getcompletion(line, 'cmdline')
    end,
  })
end

-- A function a plugin replaces, such as vim.ui.select, stands in for the
-- plugin until it's called.
local function replace(name, spec, path)
  local held, key = holder(path)
  spec.originals[path] = held[key]
  local function stand_in_for(...)
    M.load(name)
    return held[key](...)
  end
  spec.replaced[path] = stand_in_for
  held[key] = stand_in_for
end

-- Says how a plugin waits. `how` may hold `on`, the events that wake it;
-- `soon`, to load once the first screen is drawn; `actions`, the actions
-- it provides; `commands`, ones its config makes beside its plugin's own;
-- `replaces`, functions it puts its own in place of; `needs`, plugins to
-- load before it; and `also`, plugins to load after it.
function M.add(name, how, config)
  if loaded[name] then
    if config then
      configure(name, config)
    end
    return
  end
  local spec = vim.tbl_extend('force', how, {
    config = config,
    stand_ins = {},
    waits = {},
    replaced = {},
    originals = {},
  })
  specs[name] = spec
  for _, action in ipairs(how.actions or {}) do
    providers[action] = name
  end
  local brings = waiting[name] or { commands = {} }
  for _, command in ipairs(vim.list_extend(vim.deepcopy(brings.commands), how.commands or {})) do
    if vim.fn.exists(':' .. command) ~= 2 then
      stand_in(name, command)
      table.insert(spec.stand_ins, command)
    end
  end
  for _, path in ipairs(how.replaces or {}) do
    replace(name, spec, path)
  end
  if how.on then
    table.insert(
      spec.waits,
      vim.api.nvim_create_autocmd(how.on, {
        group = group,
        callback = function(event)
          local before = groups(event.event)
          M.load(name)
          for id in pairs(groups(event.event)) do
            if not before[id] then
              vim.api.nvim_exec_autocmds(event.event, {
                group = id,
                buffer = event.buf,
                modeline = false,
                data = event.data,
              })
            end
          end
        end,
      })
    )
  end
  if how.soon then
    table.insert(soon, name)
  end
end

-- Runs fn with a module once it first loads, or now if it has, so what
-- sets a module up waits as long as the module does.
function M.when(module, fn)
  if package.loaded[module] then
    return fn(package.loaded[module])
  end
  package.preload[module] = function(...)
    package.preload[module] = nil
    for index = 2, #package.loaders do
      local found = package.loaders[index](module)
      if type(found) == 'function' then
        local value = found(...)
        package.loaded[module] = value
        fn(value)
        return value
      end
    end
    error('no module ' .. module)
  end
end

-- Whether a plugin waiting holds a module's own file.
local function holds(name, module)
  local path = '/lua/' .. (module:gsub('%.', '/'))
  for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/opt/' .. name, false, true)) do
    if vim.uv.fs_stat(dir .. path .. '.lua') or vim.uv.fs_stat(dir .. path .. '/init.lua') then
      return true
    end
  end
  return false
end

-- A module of a plugin not yet loaded loads the plugin holding it. This
-- searcher comes last, so it only hears of modules nothing on the runtime
-- path has.
table.insert(package.loaders, function(module)
  local name
  for _, candidate in ipairs(owners[module:match('^[^.]+')] or {}) do
    if not loaded[candidate] and holds(candidate, module) then
      name = candidate
      break
    end
  end
  if name == nil then
    return nil
  end
  M.load(name)
  -- The plugin's config may have required this very module as it set up;
  -- that copy is the one set up, so it's the one to answer with.
  if package.loaded[module] ~= nil then
    local value = package.loaded[module]
    return function()
      return value
    end
  end
  for index = 1, #package.loaders - 1 do
    local found = package.loaders[index](module)
    if type(found) == 'function' then
      return found
    end
  end
  return nil
end)

-- The plugins that work in the background load once the first screen is
-- drawn, one at a time between keys, so the screen comes first and typing
-- never waits on them.
vim.api.nvim_create_autocmd('UIEnter', {
  group = group,
  once = true,
  callback = function()
    local function next()
      local name = table.remove(soon, 1)
      if name then
        M.load(name)
        vim.schedule(next)
      end
    end
    vim.schedule(next)
  end,
})

return M
