sherlock.ui = sherlock.ui or {}

---@alias sherlock.ui.FlexDirection
---| "row"
---| "column"
---| "row_reverse"
---| "column_reverse"

---@alias sherlock.ui.Align
---| "start"
---| "end"
---| "flex_start"
---| "flex_end"
---| "center"
---| "baseline"
---| "stretch"

---@alias sherlock.ui.Justify
---| "start"
---| "end"
---| "flex_start"
---| "flex_end"
---| "center"
---| "stretch"
---| "space_between"
---| "space_evenly"
---| "space_around"

---@alias sherlock.ui.TextAlign
---| "left"
---| "right"
---| "center"

---@class sherlock.ui.Node
---@field _type string
---@field _props table
---@field _style table
---@field _children sherlock.ui.Node[]
local Node = {}
Node.__index = Node

---@param node_type string
---@param props table?
---@return sherlock.ui.Node
function Node.new(node_type, props)
    local self = setmetatable({}, Node) --[[@as sherlock.ui.Node]]
    self._type = node_type
    self._props = props or {}
    self._style = {}
    self._children = {}
    return self
end

---@param node sherlock.ui.Node
---@return sherlock.ui.Node
function Node:child(node)
    table.insert(self._children, node)
    return self
end

---@param style_table table
---@return sherlock.ui.Node
function Node:style(style_table)
    for k, v in pairs(style_table) do
        self._style[k] = v
    end
    return self
end

--- Called with the tile id when the node is clicked.
---@param callback fun(tile_id: string)
---@return sherlock.ui.Node
function Node:on_click(callback)
    self._props.on_click = callback
    return self
end

--- Tile-level: runs when the tile is activated (Enter). Only used on a
--- tile's root node.
---@param callback fun(tile_id: string)
---@return sherlock.ui.Node
function Node:on_activate(callback)
    self._props.on_activate = callback
    return self
end

--- Tile-level: hide the tile from results (e.g. from `on_query`). Only
--- used on a tile's root node.
---@param hidden boolean?
---@return sherlock.ui.Node
function Node:hide(hidden)
    self._props.hidden = hidden ~= false
    return self
end

---@class sherlock.ui.ActionOpts
---@field icon string? icon name
---@field exit boolean? close the launcher after running

--- Tile-level: adds a context-menu entry. Only used on a tile's root node.
---@param name string
---@param callback fun(tile_id: string)
---@param opts sherlock.ui.ActionOpts?
---@return sherlock.ui.Node
function Node:action(name, callback, opts)
    opts = opts or {}
    self._props.actions = self._props.actions or {}
    table.insert(self._props.actions, {
        name = name,
        run = callback,
        icon = opts.icon,
        exit = opts.exit,
    })
    return self
end

---@return table
function Node:build()
    local children = {}
    for i, c in ipairs(self._children) do
        children[i] = c.build and c:build() or c
    end

    return {
        type = self._type,
        content = self._props.content,
        label = self._props.label,
        name = self._props.name,
        src = self._props.src,
        value = self._props.value,
        on_click = self._props.on_click,
        on_activate = self._props.on_activate,
        actions = self._props.actions,
        hidden = self._props.hidden,

        style = next(self._style) and self._style or nil,

        children = #children > 0 and children or nil,
    }
end

-- ---------------------------------------------------------------------
-- Table-constructor sugar
--
--   sherlock.ui.row {
--       gap = 8,                          -- flattened style shorthand
--       style = { padding = 4 },          -- ...or nested, both work
--       sherlock.ui.icon "search",        -- array part = children,
--       sherlock.ui.text "hi",            -- nesting is just Lua tables,
--   }                                     -- so depth is unlimited for free
--
-- Also enables re-opening an already-built node:
--   sherlock.ui.text("hi") { on_click = fn }
-- ---------------------------------------------------------------------

---@class sherlock.ui.NodeOpts
---@field style table? nested style overrides
---@field on_click fun(tile_id: string)?
---@field on_activate fun(tile_id: string)? tile root only
---@field actions { name: string, run: fun(tile_id: string), icon: string?, exit: boolean? }[]? tile root only
--- any recognized style key (gap, padding, background, ...) may also be
--- set directly at the top level of this table instead of nesting it
--- under `style`; see STYLE_KEYS below for the full set.
--- Array-part entries (no key) are treated as children.

local STYLE_KEYS = {
    flex = true,
    width = true, height = true,
    min_width = true, min_height = true, max_width = true, max_height = true,
    margin_x = true, margin_y = true,
    padding = true, padding_x = true, padding_y = true, margin = true,
    gap = true, flex_grow = true, flex_shrink = true,
    background = true, border_color = true, border_width = true,
    corner_radii = true, opacity = true, color = true,
    font_size = true, font_family = true, text_align = true,
    flex_direction = true, align_items = true, justify_content = true,
    hover = true,
}

---@param node sherlock.ui.Node
---@param opts sherlock.ui.NodeOpts|sherlock.ui.Node[]|nil
---@return sherlock.ui.Node
local function apply_opts(node, opts)
    if type(opts) ~= "table" then
        return node
    end
    for i, child in ipairs(opts) do
        node:child(child)
    end
    if opts.style then
        node:style(opts.style)
    end
    for k in pairs(STYLE_KEYS) do
        if opts[k] ~= nil then
            node._style[k] = opts[k]
        end
    end
    if opts.on_click then
        node:on_click(opts.on_click)
    end
    if opts.hidden ~= nil then
        node:hide(opts.hidden)
    end
    if opts.on_activate then
        node:on_activate(opts.on_activate)
    end
    for _, a in ipairs(opts.actions or {}) do
        node:action(a.name, a.run, a)
    end
    return node
end

-- lets a returned Node be "called" again to merge in more opts, e.g.
-- sherlock.ui.text("hi") { style = { color = "red" } }
Node.__call = function(self, opts)
    return apply_opts(self, opts)
end

-- ---------------------------------------------------------------------
-- Constructors
-- ---------------------------------------------------------------------

---@param opts (sherlock.ui.NodeOpts|sherlock.ui.Node[])?
---@return sherlock.ui.Node
function sherlock.ui.row(opts)
    local node = Node.new("container")
    node:flex_direction("row")
    return apply_opts(node, opts)
end

---@param opts (sherlock.ui.NodeOpts|sherlock.ui.Node[])?
---@return sherlock.ui.Node
function sherlock.ui.column(opts)
    local node = Node.new("container")
    node:flex_direction("column")
    return apply_opts(node, opts)
end

---@param opts (sherlock.ui.NodeOpts|sherlock.ui.Node[])?
---@return sherlock.ui.Node
function sherlock.ui.container(opts)
    local node = Node.new("container")
    return apply_opts(node, opts)
end

---@class sherlock.ui.TextOpts : sherlock.ui.NodeOpts
---@field content string?

---@param content string|sherlock.ui.TextOpts
---@return sherlock.ui.Node
function sherlock.ui.text(content)
    if type(content) == "table" then
        local node = Node.new("text", { content = content.content })
        return apply_opts(node, content)
    end
    return Node.new("text", { content = content })
end

---@class sherlock.ui.IconOpts : sherlock.ui.NodeOpts
---@field name string?

---@param name string|sherlock.ui.IconOpts
---@return sherlock.ui.Node
function sherlock.ui.icon(name)
    if type(name) == "table" then
        local node = Node.new("icon", { name = name.name })
        return apply_opts(node, name)
    end
    return Node.new("icon", { name = name })
end

---@class sherlock.ui.ButtonOpts : sherlock.ui.NodeOpts
---@field label string?

---@param label string|sherlock.ui.ButtonOpts
---@return sherlock.ui.Node
function sherlock.ui.button(label)
    if type(label) == "table" then
        local node = Node.new("button", { label = label.label })
        return apply_opts(node, label)
    end
    return Node.new("button", { label = label })
end

--- Image from a file path (`~` expanded) or an http(s) URL.
---@param src string|table
---@return sherlock.ui.Node
function sherlock.ui.image(src)
    if type(src) == "table" then
        local node = Node.new("image", { src = src.src })
        return apply_opts(node, src)
    end
    return Node.new("image", { src = src })
end

--- Progress bar filled to `value` (0..1). `background` styles the track,
--- `color` the fill.
---@param value number|table
---@return sherlock.ui.Node
function sherlock.ui.progress(value)
    if type(value) == "table" then
        local node = Node.new("progress", { value = value.value })
        return apply_opts(node, value)
    end
    return Node.new("progress", { value = value })
end

--- Thin horizontal line.
---@param opts table?
---@return sherlock.ui.Node
function sherlock.ui.divider(opts)
    return apply_opts(Node.new("divider"), opts)
end

--- Fills the remaining space in a row/column.
---@param opts table?
---@return sherlock.ui.Node
function sherlock.ui.spacer(opts)
    return apply_opts(Node.new("spacer"), opts)
end

-- ---------------------------------------------------------------------
-- Fluent style setters (kept for power users / imperative building --
-- the table-constructor sugar above is just an alternate front end
-- onto the same Node object)
-- ---------------------------------------------------------------------

---@param v number
---@return sherlock.ui.Node
function Node:width(v)
    self._style.width = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:height(v)
    self._style.height = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:padding(v)
    self._style.padding = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:padding_x(v)
    self._style.padding_x = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:padding_y(v)
    self._style.padding_y = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:margin(v)
    self._style.margin = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:gap(v)
    self._style.gap = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:grow(v)
    self._style.flex_grow = v
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:shrink(v)
    self._style.flex_shrink = v
    return self
end

---@param color string
---@return sherlock.ui.Node
function Node:bg(color)
    self._style.background = color
    return self
end

---@param color string
---@param width number?
---@return sherlock.ui.Node
function Node:border(color, width)
    self._style.border_color = color
    self._style.border_width = width or 1
    return self
end

---@param radius number
---@return sherlock.ui.Node
function Node:rounded(radius)
    self._style.corner_radii = radius
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:opacity(v)
    self._style.opacity = v
    return self
end

---@param color string
---@return sherlock.ui.Node
function Node:color(color)
    self._style.color = color
    return self
end

---@param v number
---@return sherlock.ui.Node
function Node:font_size(v)
    self._style.font_size = v
    return self
end

---@param family string
---@return sherlock.ui.Node
function Node:font_family(family)
    self._style.font_family = family
    return self
end

---@param v sherlock.ui.TextAlign
---@return sherlock.ui.Node
function Node:text_align(v)
    self._style.text_align = v
    return self
end

---@param v sherlock.ui.FlexDirection
---@return sherlock.ui.Node
function Node:flex_direction(v)
    self._style.flex_direction = v
    return self
end

---@param v sherlock.ui.Align
---@return sherlock.ui.Node
function Node:align_items(v)
    self._style.align_items = v
    return self
end

---@param v sherlock.ui.Justify
---@return sherlock.ui.Node
--- Style overrides while hovered, e.g. `:hover { background = "bg_selected" }`.
---@param style_table table
---@return sherlock.ui.Node
function Node:hover(style_table)
    self._style.hover = style_table
    return self
end

function Node:justify_content(v)
    self._style.justify_content = v
    return self
end

local tile_callbacks = {}

-- Before a tile's node is sent, `_prepare` replaces every `on_click` 
-- function with an index into that tile's callback list. 
-- Each send replaces the list, so closures from older renders are released.
local function prepare(tile_id, node)
    local callbacks = {}
    local function register(fn)
        if type(fn) ~= "function" then
            return nil
        end
        callbacks[#callbacks + 1] = fn
        return #callbacks
    end
    local function walk(n)
        if type(n) ~= "table" then
            return n
        end
        if n.build then
            n = n:build()
        end
        -- Copy so the caller's table keeps its functions for reuse.
        local out = {}
        for k, v in pairs(n) do
            out[k] = v
        end
        out.on_click = register(out.on_click)
        out.on_activate = register(out.on_activate)
        if type(out.actions) == "table" and #out.actions > 0 then
            local actions = {}
            for _, a in ipairs(out.actions) do
                actions[#actions + 1] = {
                    name = a.name,
                    icon = a.icon,
                    exit = a.exit == true,
                    run = register(a.run),
                }
            end
            out.actions = actions
        else
            out.actions = nil
        end
        if type(out.children) == "table" then
            local children = {}
            for i, c in ipairs(out.children) do
                children[i] = walk(c)
            end
            out.children = children
        end
        return out
    end
    local prepared = walk(node)
    tile_callbacks[tile_id] = callbacks
    return prepared
end

sherlock._prepare = prepare

local row_ids = {}

function sherlock._prepare_rows(rows)
    local out, new_ids = {}, {}
    for i, row in ipairs(rows) do
        local node, id, search = row, nil, nil
        if type(row) == "table" and not row.build and row.node ~= nil then
            node, id, search = row.node, row.id, row.search
        end
        id = id or ("result:" .. i)
        out[i] = { id = id, search = search, node = prepare(id, node) }
        new_ids[id] = true
    end
    for id in pairs(row_ids) do
        if not new_ids[id] then
            tile_callbacks[id] = nil
        end
    end
    row_ids = new_ids
    return out
end

function sherlock._invoke(tile_id, index)
    local callbacks = tile_callbacks[tile_id]
    local callback = callbacks and callbacks[index]
    if callback then
        return callback(tile_id)
    end
end

local raw_update = sherlock.ui.update
if raw_update then
    function sherlock.ui.update(tile_id, node)
        return raw_update(tile_id, prepare(tile_id, node))
    end
end
