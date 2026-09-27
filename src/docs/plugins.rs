use indoc::indoc;
use md_rs::{
    cached_component,
    components::{
        Component, ParentComponentExt,
        code_block::codeblock,
        container::Container,
        details::details,
        heading::h4,
        list::{ListStyle, md_list},
        raw::{Raw, raw},
        span::{bold, br, code, html_strong, italic, keybind},
        table::table,
    },
    md, p,
};

/// Guide for plugin authors, shown in the plugin launcher's docs.
pub fn plugin_guide_section() -> Raw {
    cached_component!(
        24 * 1024,
        md!(details()
            .summary(html_strong("Writing Plugins:"))
            .child(md!(
                p!(
                    "Plugins are Lua scripts that add their own tiles to Sherlock. \
                A plugin can show live data, react to clicks and keys, add \
                context-menu actions and answer search queries."
                ),
                guide_quick_start(),
                guide_lifecycle(),
                guide_ui(),
                guide_updates(),
                guide_interaction(),
                guide_navigation(),
                guide_search(),
                guide_sandbox(),
            )))
    )
}

fn guide_quick_start() -> Container {
    md!(
        h4("Quick start"),
        md_list()
            .style(ListStyle::Ordered)
            .item(p!(
                "Run",
                code("sherlock plugin-init"),
                "inside your plugin directory. It writes the API stubs",
                code("init.lua"),
                "and",
                code("ui.lua"),
                "to Sherlock's cache and a",
                code(".luarc.json"),
                "pointing at them, so the Lua language server can autocomplete the API."
            ))
            .item(md!(
                p!(
                    "Create the plugin, e.g.",
                    code("~/.config/sherlock/plugins/hello/init.lua"),
                    ":"
                ),
                codeblock().lang("lua").content(indoc! {r#"
                    local ui = sherlock.ui

                    function tiles()
                      return { { id = "hello", node = ui.text "Hello from Lua" } }
                    end
                "#})
            ))
            .item(p!(
                "Register it as a",
                code("plugin"),
                "launcher (see the example below). The",
                bold("Reload"),
                "action re-runs the plugin after you edit it."
            )),
        p!(
            "A plugin loaded from",
            code("init.lua"),
            "is named after its directory, otherwise after its file name."
        ),
    )
}

fn guide_lifecycle() -> Container {
    md!(
        h4("Lifecycle functions"),
        p!("All functions are optional except", code("tiles"), "."),
        table()
            .headers(["Function", "When it runs", "Returns"])
            .row(["`init(theme)`", "Once after loading, before `tiles`.", "–",])
            .row([
                "`tiles()`",
                "After `init`. Defines the static tiles.",
                "list of `{ id, node, search? }`",
            ])
            .row([
                "`refresh(tile_id)`",
                "When a tile is shown.",
                "a node, or nothing to keep the current one",
            ])
            .row([
                "`live(tile_id)`",
                "Once per tile as a background loop. Stopped on reload.",
                "–",
            ])
            .row([
                "`on_query(query)`",
                "Every time the search text changes (lowercased).",
                "result rows, or `nil`",
            ]),
        p!(
            code("init"),
            ",",
            code("tiles"),
            "and",
            code("refresh"),
            "time out after 10 seconds while waiting (e.g. on HTTP). \
            Errors from plugin code are shown in Sherlock's message view. \
            Until",
            code("init"),
            "and",
            code("tiles"),
            "finish, a",
            italic("Loading…"),
            "tile is shown, so a slow plugin never blocks the UI."
        ),
    )
}

fn guide_ui() -> Container {
    md!(
        h4("Building UI"),
        p!(
            code("sherlock.ui"),
            "builds nodes. Nodes nest with Lua tables; style keys can be given \
            directly or under",
            code("style"),
            ":"
        ),
        codeblock().lang("lua").content(indoc! {r#"
            local ui = sherlock.ui
            local node = ui.row {
              gap = 8, padding = 8,
              ui.icon "weather-clear",
              ui.column {
                ui.text { content = "Sunny", font_size = 16 },
                ui.text { content = "21°C", color = "text_muted" },
              },
            }
        "#}),
        p!(
            "Every node also has fluent setters like",
            code(":padding(8)"),
            ",",
            code(":bg(\"accent\")"),
            "or",
            code(":rounded(4)"),
            ", and can be called again to merge options:",
            code("ui.text(\"hi\") { color = \"error\" }"),
            "."
        ),
        table()
            .headers(["Node", "Description"])
            .row([
                "`ui.row {…}`, `ui.column {…}`, `ui.container {…}`",
                "Layout containers.",
            ])
            .row(["`ui.text \"…\"`", "Text."])
            .row(["`ui.icon \"name\"`", "Icon from the icon theme."])
            .row([
                "`ui.image \"path\"`",
                "Image from a file (`~` expanded).",
            ])
            .row([
                "`ui.button \"label\"`",
                "Button-styled text, usually combined with `on_click`.",
            ])
            .row([
                "`ui.progress(0.4)`",
                "Progress bar (0–1). `background` styles the track, `color` the fill.",
            ])
            .row(["`ui.divider()`", "Thin horizontal line."])
            .row(["`ui.spacer()`", "Fills the remaining space in a row or column."]),
        table()
            .headers(["Style group", "Keys"])
            .row([
                "Layout",
                "`flex` `flex_direction` `flex_grow` `flex_shrink` `align_items` `justify_content` `gap`",
            ])
            .row([
                "Size",
                "`width` `height` `min_width` `min_height` `max_width` `max_height`",
            ])
            .row([
                "Spacing",
                "`padding` `padding_x` `padding_y` `margin` `margin_x` `margin_y`",
            ])
            .row([
                "Visual",
                "`background` `border_color` `border_width` `corner_radii` `opacity`",
            ])
            .row(["Text", "`color` `font_family` `font_size` `text_align`"])
            .row(["States", "`hover` `focus` (nested style tables)"]),
        p!(
            "Any flex property makes the node a flex container unless",
            code("flex = false"),
            ".",
            code("hover"),
            "applies while the pointer is over the node,",
            code("focus"),
            "while it is the keyboard-focused item (see",
            italic("Navigation"),
            ")."
        ),
        p!(
            bold("Colors"),
            "are hex",
            code("#rrggbb"),
            "/",
            code("#rrggbbaa"),
            "or a theme token:",
            code("text"),
            code("text_secondary"),
            code("text_muted"),
            code("bg"),
            code("bg_muted"),
            code("bg_selected"),
            code("border"),
            code("accent"),
            code("success"),
            code("warning"),
            code("error"),
            code("info"),
            "."
        ),
    )
}

fn guide_updates() -> Container {
    md!(
        h4("Updating tiles"),
        p!(
            code("sherlock.ui.update(tile_id, node)"),
            "replaces a tile's content at any time. It needs the",
            code("ui"),
            "capability."
        ),
        codeblock().lang("lua").content(indoc! {r#"
            function live(tile_id)
              local n = 0
              while true do
                n = n + 1
                sherlock.ui.update(tile_id, sherlock.ui.text("Tick " .. n))
                sherlock.time.sleep_ms(1000)
              end
            end
        "#}),
    )
}

fn guide_interaction() -> Container {
    md!(
        h4("Interaction"),
        p!("Callbacks receive the tile id."),
        md_list()
            .style(ListStyle::Dash)
            .item(p!(
                bold("on_click"),
                "on any node. Clicking it runs the callback."
            ))
            .item(p!(
                bold("on_activate"),
                "on a tile's root node. Runs on",
                keybind("Enter"),
                "when the tile is selected and no inner item is focused."
            ))
            .item(p!(
                bold(":action(name, fn, { icon?, exit? })"),
                "on a tile's root node. Adds a context-menu entry;",
                code("exit = true"),
                "closes Sherlock afterwards."
            )),
        codeblock().lang("lua").content(indoc! {r#"
            local ui = sherlock.ui
            local function view(n)
              return ui.row { ui.text("Count: " .. n) }
                :on_activate(function(id) ui.update(id, view(n + 1)) end)
                :action("Reset", function(id) ui.update(id, view(0)) end, { icon = "view-refresh" })
            end
        "#}),
        p!(
            "Callbacks are replaced every time a tile is updated, so creating \
            new closures in a loop does not leak."
        ),
    )
}

fn guide_navigation() -> Container {
    md!(
        h4("Navigation"),
        p!(
            "Nodes with",
            code("on_click"),
            "are the tile's items. When the tile is selected, the arrow keys \
            move between them in document order and",
            keybind("Enter"),
            "runs the focused item's",
            code("on_click"),
            ". Moving past the last item continues to the next result; moving \
            back past the first returns focus to the tile."
        ),
        p!(
            code("nav"),
            "on the tile's root node picks the keys:",
            code("\"horizontal\""),
            "(default, ←/→),",
            code("\"vertical\""),
            "(↑/↓) or",
            code("\"both\""),
            "."
        ),
        codeblock().lang("lua").content(indoc! {r#"
            local ui = sherlock.ui
            local function btn(label, fn)
              return ui.button(label) { padding_x = 10, focus = { background = "accent" }, on_click = fn }
            end

            local function prev_track() sherlock.log.info("prev") end
            local function toggle() sherlock.log.info("toggle") end
            local function next_track() sherlock.log.info("next") end

            function tiles()
              return { { id = "media", node = ui.row {
                gap = 8, ui.text "Now playing", ui.spacer(),
                btn("⏮", prev_track), btn("⏯", toggle), btn("⏭", next_track),
              } } }
            end
        "#}),
    )
}

fn guide_search() -> Container {
    md!(
        h4("Search"),
        p!(
            "By default a tile is matched against its",
            code("search"),
            "text from",
            code("tiles()"),
            ", falling back to the launcher name. A plugin that defines",
            code("on_query(query)"),
            "filters itself instead:"
        ),
        md_list()
            .style(ListStyle::Dash)
            .item(p!(
                bold("Hide/show tiles:"),
                "set",
                code(":hide()"),
                "or",
                code("hidden = true"),
                "on a tile's root node and",
                code("ui.update"),
                "it."
            ))
            .item(p!(
                bold("Return result rows:"),
                "rows are shown after the static tiles and replaced on every \
                query. A row is a node, or",
                code("{ id?, search?, node }"),
                "with ids defaulting to",
                code("result:1"),
                ",",
                code("result:2"),
                ", …. Return",
                code("{}"),
                "to clear them,",
                code("nil"),
                "to leave them unchanged."
            )),
        codeblock().lang("lua").content(indoc! {r#"
            local ui = sherlock.ui
            local apps = { "Firefox", "Files", "Terminal" }

            function tiles() return {} end

            function on_query(q)
              if q == "" then return {} end
              local rows = {}
              for _, name in ipairs(apps) do
                if name:lower():find(q, 1, true) then
                  rows[#rows + 1] = ui.text(name):on_activate(function() sherlock.clipboard.set(name) end)
                end
              end
              return rows
            end
        "#}),
        p!(
            "The example needs the",
            code("clipboard"),
            "capability. A call for an older query is cancelled when a newer \
            query arrives."
        ),
    )
}

fn guide_sandbox() -> Container {
    md!(
        h4("Sandbox"),
        p!(
            "Each plugin runs in its own environment with the safe Lua base \
            functions, private copies of",
            code("table"),
            ",",
            code("string"),
            ",",
            code("math"),
            ",",
            code("coroutine"),
            ", a text-only",
            code("load"),
            "and the",
            code("sherlock"),
            "API allowed by its capabilities (see below). Not available:",
            code("io"),
            code("os"),
            code("debug"),
            code("package"),
            code("dofile"),
            code("loadfile"),
            "and C modules."
        ),
        p!(
            code("require(\"name\")"),
            "loads",
            code("name.lua"),
            "or",
            code("name/init.lua"),
            "from the plugin's own directory only."
        ),
    )
}
