use std::collections::HashMap;

use gpui::SharedString;

use super::command_launch::spawn_detached;
use crate::{
    config::ConfigGuard,
    utils::{errors::SherlockMessage, url_detection::is_url},
};

pub fn websearch(
    mut engine: &str,
    query: &str,
    browser: Option<&str>,
    variables: &[(SharedString, SharedString)],
) -> Result<(), SherlockMessage> {
    if is_url(query) {
        engine = "plain";
    }
    let engines: HashMap<&str, &str> = HashMap::from([
        ("google", "https://www.google.com/search?q={keyword}"),
        ("bing", "https://www.bing.com/search?q={keyword}"),
        ("duckduckgo", "https://duckduckgo.com/?q={keyword}"),
        ("yahoo", "https://search.yahoo.com/search?p={keyword}"),
        ("baidu", "https://www.baidu.com/s?wd={keyword}"),
        ("yandex", "https://yandex.com/search/?text={keyword}"),
        ("ask", "https://www.ask.com/web?q={keyword}"),
        ("ecosia", "https://www.ecosia.org/search?q={keyword}"),
        ("qwant", "https://www.qwant.com/?q={keyword}"),
        (
            "startpage",
            "https://www.startpage.com/sp/search?q={keyword}",
        ),
        ("plain", "{keyword}"),
    ]);
    let url_template = if let Some(url) = engines.get(engine) {
        url
    } else {
        engine
    };

    let mut browser = match browser {
        Some(b) => b.to_string(),
        None => ConfigGuard::read()?.default_apps.browser()?.to_string(),
    };

    let url = url_template.replace("{keyword}", &query.replace(" ", "+"));
    let command = if browser.contains("%u") {
        browser.replace("%u", &format!(r#" "{}""#, url))
    } else {
        browser.push_str(&format!(r#" "{}""#, url));
        browser
    };

    spawn_detached(&command, query, variables)
}
