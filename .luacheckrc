-- luacheck reads every Lua file through `nix fmt`, with all its warnings on
-- and none left out but the one below. Each file is read as the Lua of the
-- program that runs it, with the names that program gives it, so a name
-- nothing set is caught rather than read as nil.

-- Lines stop where stylua wraps them.
max_line_length = 100

-- Neovim runs LuaJIT, and its config writes into the `vim` table it's given.
-- Snacks is the global the snacks plugin sets for itself.
files['modules/neovim'] = { std = 'luajit', globals = { 'vim' }, read_globals = { 'Snacks' } }

-- The dashboard's picture keeps its trailing spaces, since snacks centres
-- each line by its width, and trimming them would skew the picture.
files['modules/neovim/plugins/snacks.lua'] = { ignore = { '613' } }

-- mpv runs the camera's viewfinder script in Lua 5.2 and gives it `mp`.
files['modules/camera'] = { std = 'lua52', read_globals = { 'mp' } }

-- mpv's own scripts, the player's, run the same way.
files['modules/mpv'] = { std = 'lua52', read_globals = { 'mp' } }

-- swayimg runs LuaJIT, and its config writes into the `swayimg` table.
files['modules/swayimg'] = { std = 'luajit', globals = { 'swayimg' } }

-- WezTerm runs Lua 5.4, and its config gets everything through require.
files['modules/wezterm'] = { std = 'lua54' }
