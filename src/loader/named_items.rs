use std::{fmt, marker::PhantomData, sync::Arc};

use gpui::SharedString;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};

use crate::{
    loader::{
        resolve_icon_path,
        utils::{ApplicationAction, construct_search},
    },
    ui::{
        launcher::context_menu::ContextMenuAction,
        runtime_arguments::RuntimeArgument,
        widgets::list_item::{ItemMeta, ListItem, Payload},
    },
};

#[derive(Deserialize)]
struct EntrySerde {
    exec: SharedString,
    #[serde(default)]
    search_string: String,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    actions: Vec<ActionSerde>,
    #[serde(default, rename = "variables")]
    vars: Vec<RuntimeArgument>,
}

#[derive(Deserialize)]
struct ActionSerde {
    name: SharedString,
    #[serde(default)]
    exec: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    method: String,
    #[serde(default)]
    exit: Option<bool>,
}

/// Deserializes a config map `{ "<name>": { exec, ... } }` into list items.
/// The map key is both the display name and the stable identity (`ItemKey::command`),
/// so renaming via alias later never loses history.
///
/// ```ignore
/// deserialize_named_items(de, &launcher, use_kw, |exec| CommandPayload { exec })
/// deserialize_named_items(de, &launcher, use_kw, |short| CategoryPayload { short })
/// ```
pub fn deserialize_named_items<'de, D, P, F>(
    deserializer: D,
    make_payload: F,
) -> Result<Vec<ListItem<P>>, D::Error>
where
    D: Deserializer<'de>,
    P: Payload,
    F: Fn(SharedString) -> P,
{
    struct ItemMapVisitor<P, F> {
        make_payload: F,
        _p: PhantomData<P>,
    }

    impl<'de, P: Payload, F: Fn(SharedString) -> P> Visitor<'de> for ItemMapVisitor<P, F> {
        type Value = Vec<ListItem<P>>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map of entries keyed by name")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut items = Vec::with_capacity(map.size_hint().unwrap_or(0));

            while let Some((name, entry)) = map.next_entry::<SharedString, EntrySerde>()? {
                let icon = entry.icon.as_deref().and_then(resolve_icon_path);

                let actions: Arc<[Arc<ContextMenuAction>]> = entry
                    .actions
                    .into_iter()
                    .map(|a| {
                        Arc::new(ContextMenuAction::App(ApplicationAction {
                            name: a.name,
                            icon: a
                                .icon
                                .as_deref()
                                .and_then(resolve_icon_path)
                                .or_else(|| icon.clone()),
                            method: a.method,
                            exec: a.exec,
                            exit: a.exit.unwrap_or(true),
                        }))
                    })
                    .collect();

                // use_keywords is deliberately true, since those are created by the user themselves
                let search = construct_search(Some(&name), &entry.search_string, true);

                items.push(ListItem {
                    meta: ItemMeta {
                        search: search.into(),
                        icon,
                        actions,
                        vars: entry.vars.into(),
                        name,
                    },
                    payload: (self.make_payload)(entry.exec),
                });
            }
            Ok(items)
        }
    }

    deserializer.deserialize_map(ItemMapVisitor {
        make_payload,
        _p: PhantomData,
    })
}
