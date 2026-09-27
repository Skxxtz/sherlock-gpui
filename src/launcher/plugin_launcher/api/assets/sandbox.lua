-- Builds isolated plugin environments. Evaluated once with full globals;
-- returns `make_env(root)`, which creates a fresh environment per plugin.

-- Hide the shared string library behind `getmetatable("")`.
getmetatable("").__metatable = false

local SAFE_FUNCS = {
    "assert", "error", "ipairs", "next", "pairs", "pcall", "print",
    "rawequal", "rawget", "rawlen", "rawset", "select", "setmetatable",
    "getmetatable", "tonumber", "tostring", "type", "xpcall",
}
-- Copied per plugin so a plugin patching e.g. `string.format` can't affect others.
local SAFE_LIBS = { "table", "string", "math", "coroutine" }

local base_load, base_loadfile = load, loadfile

local function valid_module_name(name)
    return type(name) == "string"
        and name:find("^[%w_%-%.]+$") ~= nil
        and name:find("%.%.") == nil
        and name:sub(1, 1) ~= "."
        and name:sub(-1) ~= "."
end

return function(root)
    local env = {}
    for _, k in ipairs(SAFE_FUNCS) do
        env[k] = _G[k]
    end
    for _, lib in ipairs(SAFE_LIBS) do
        local copy = {}
        for k, v in pairs(_G[lib]) do
            copy[k] = v
        end
        env[lib] = copy
    end
    env._G = env
    env._VERSION = _VERSION

    -- Text chunks only (bytecode is unsafe), bound to this plugin's env.
    env.load = function(chunk, name, _mode, chunk_env)
        return base_load(chunk, name, "t", chunk_env or env)
    end

    -- `require` resolves only inside the plugin directory, with its own cache.
    local loaded = {}
    env.require = function(name)
        if not valid_module_name(name) then
            error("invalid module name: " .. tostring(name), 2)
        end
        local cached = loaded[name]
        if cached ~= nil then
            return cached
        end
        local rel = name:gsub("%.", "/")
        local chunk, err
        for _, candidate in ipairs({ rel .. ".lua", rel .. "/init.lua" }) do
            chunk, err = base_loadfile(root .. "/" .. candidate, "t", env)
            if chunk then
                break
            end
        end
        if not chunk then
            error("module '" .. name .. "' not found: " .. tostring(err), 2)
        end
        local result = chunk(name)
        if result == nil then
            result = true
        end
        loaded[name] = result
        return result
    end

    return env
end
