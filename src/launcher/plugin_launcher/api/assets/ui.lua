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

---@param callback fun()
---@return sherlock.ui.Node
function Node:on_click(callback)
    local id = sherlock._register_callback(callback)
    self._props.on_click = id
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
        direction = self._props.direction,
        content = self._props.content,
        label = self._props.label,
        name = self._props.name,
        on_click = self._props.on_click,

        style = next(self._style) and self._style or nil,

        children = #children > 0 and children or nil,
    }
end

---@return sherlock.ui.Node
function sherlock.ui.row()
    local node = Node.new("container")
    node:flex_direction("row")
    return node
end

---@return sherlock.ui.Node
function sherlock.ui.column()
    local node = Node.new("container")
    node:flex_direction("column")
    return node
end

---@param content string
---@return sherlock.ui.Node
function sherlock.ui.text(content)
    return Node.new("text", { content = content })
end

---@param name string
---@return sherlock.ui.Node
function sherlock.ui.icon(name)
    return Node.new("icon", { name = name })

end

---@param label string
---@return sherlock.ui.Node
function sherlock.ui.button(label)
    return Node.new("button", { label = label })
end

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
---@param width number
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
function Node:justify_content(v)
    self._style.justify_content = v
    return self
end

