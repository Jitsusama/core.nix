-- Every name Neovim takes for a command of its own, and the command it
-- runs, as JSON: :red is redo and :lock is lockmarks. Neovim with no config
-- runs it once as keys-speak-the-language builds (checks.nix), so the key
-- language's commands are held to what Neovim itself resolves, as
-- exists(':name') does, rather than to a list kept by hand.
local names = {}
for _, command in ipairs(vim.fn.getcompletion('', 'command')) do
  for length = 1, #command do
    local name = command:sub(1, length)
    local resolved = vim.fn.fullcommand(name)
    if resolved ~= '' then
      names[name] = resolved
    end
  end
end
-- A table's keys come out in no fixed order, so they're written sorted, one
-- to a line, to build the same file every time.
local sorted = vim.tbl_keys(names)
table.sort(sorted)
local lines = {}
for _, name in ipairs(sorted) do
  lines[#lines + 1] = vim.json.encode(name) .. ': ' .. vim.json.encode(names[name])
end
io.stdout:write('{\n' .. table.concat(lines, ',\n') .. '\n}\n')
