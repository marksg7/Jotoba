use error::Error;
use itertools::Itertools;
use japanese::JapaneseExt;
use search::engine::{self, SearchTask};

/// Returns a list of radicals based on the radical-search `query`
pub fn search(query: &str) -> Vec<char> {
    if japanese::JapaneseExt::has_kanji(query) {
        return kanji_search(query);
    }

    kana_search(query).unwrap_or_default()
}

/// Takes all kanji from `query` and returns a list of all unique radicals to build all kanji
/// picked from `query`
fn kanji_search(query: &str) -> Vec<char> {
    let kanji_retr = resources::get().kanji();

    query
        .chars()
        .filter_map(|kanji| kanji_retr.by_literal(kanji).and_then(|i| i.parts.as_ref()))
        .flatten()
        .copied()
        .unique()
        .collect()
}

/// Does a kana word-search and returns some likely radicals for the given query
fn kana_search(query: &str) -> Result<Vec<char>, Error> {
    let mut search_task: SearchTask<engine::words::native::Engine> =
        SearchTask::new(&query).limit(3).threshold(0.8f32);

    let original_query = query.to_string();
    search_task.set_order_fn(move |word, rel, q_str, _| {
        search::word::order::japanese_search_order(word, rel, q_str, Some(&original_query))
    });

    let kanji_retr = resources::get().kanji();
    let res = search_task
        .find()?
        .item_iter()
        .map(|i| {
            i.get_reading()
                .reading
                .chars()
                .filter(|i| i.is_kanji())
                .collect::<Vec<char>>()
        })
        .flatten()
        .unique()
        .filter_map(|kanji| kanji_retr.by_literal(kanji).and_then(|i| i.parts.as_ref()))
        .flatten()
        .unique()
        .copied()
        .take(10)
        .collect::<Vec<_>>();

    Ok(res)
}
