use md_rs::{
    cached_component,
    components::{
        Component, ParentComponentExt,
        code_block::codeblock,
        details::details,
        heading::h4,
        raw::{Raw, raw},
        span::{bold, br, code, html_strong},
    },
    md, p,
};

use crate::launcher::plugin_launcher::api::plugin_capability_docs;

pub struct PluginCapabilityFunctionDoc {
    pub name: &'static str,
    pub doc: &'static str,
}

pub struct PluginCapabilityModuleDoc {
    pub module: &'static str,
    pub functions: &'static [PluginCapabilityFunctionDoc],
}

pub fn plugin_capabilities_section() -> Raw {
    cached_component!(
        8 * 1024,
        md!(details().summary(html_strong("Capabilities:")).child(
            md!(
                p!(
                    "Plugin capabilities control what scopes the plugin can access. \
                    They are a security feature, preventing malicious plugins from \
                    accessing scopes the user doesnt explicitly allow. \
                    Otherwise, a plugin could for example create a clipboard tracker, \
                    allowing a hacker to log the users clipboard content in a remote database. \
                    Therefore, it's",
                    bold("strongly recommented."),
                    "to use a proper capability setup, allowing only necessary scopes.",
                    br(),
                    "Pass them via the",
                    code("capabilities"),
                    "arg:"
                ),
                codeblock()
                    .lang("json")
                    .content(r#"{ "capabilities": ["calc.math", "calc.units"] }"#),
            )
            .children(plugin_capability_docs().iter().map(|cap| {
                details()
                    .summary(cap.module)
                    .children(cap.functions.iter().map(|func| {
                        md!(
                            h4(p!(code(format!("{}.{}", cap.module, func.name)))),
                            func.doc
                        )
                    }))
            })),
        ))
    )
}
