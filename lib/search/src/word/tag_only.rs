use error::Error;
use resources::models::words::filter_languages;
use utils::to_option;

use crate::query::Tag;

use super::{ResultData, Search};

pub(super) fn search(search: &Search<'_>) -> Result<ResultData, Error> {
    let filter_tag = search.query.tags.iter().find(|i| i.is_empty_allowed());

    if filter_tag.is_none() {
        return Ok(ResultData::default());
    }

    match filter_tag.unwrap() {
        Tag::Jlpt(jlpt) => return jlpt_search(search, *jlpt),
        _ => return Ok(ResultData::default()),
    }
}

fn jlpt_search(search: &Search<'_>, jlpt: u8) -> Result<ResultData, Error> {
    assert!(jlpt > 0 && jlpt < 6);

    let pos_filter = to_option(search.query.get_part_of_speech_tags().copied().collect());

    let resources = resources::get();

    let word_jlpt = match resources.word_jlpt(jlpt) {
        Some(word_jlpt) => word_jlpt,
        None => return Ok(ResultData::default()),
    };

    let mut wordresults = word_jlpt
        .filter(|word| Search::word_filter(&search.query, word, &pos_filter))
        .cloned()
        .collect::<Vec<_>>();

    filter_languages(
        wordresults.iter_mut(),
        search.query.settings.user_lang,
        search.query.settings.show_english,
    );

    wordresults.sort_by(|a, b| a.get_reading().reading.cmp(&b.get_reading().reading));

    let count = wordresults.len();

    let wordresults = wordresults
        .into_iter()
        .skip(search.query.page_offset)
        .take(search.query.settings.page_size as usize)
        .collect();

    Ok(ResultData {
        count,
        words: wordresults,
        ..Default::default()
    })
}
