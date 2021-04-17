use itertools::Itertools;
use std::time::SystemTime;

use crate::{
    error::Error, japanese::JapaneseExt, models::kanji, parse::jmdict::languages::Language, DbPool,
};

use super::{result, result_order::NativeWordOrder, word, SearchMode};

const MAX_KANJI_INFO_ITEMS: usize = 4;

/// Search among all data based on the input query
pub async fn search(db: &DbPool, query: &str) -> Result<Vec<result::Item>, Error> {
    let start = SystemTime::now();

    // Perform (word) searches asynchronously
    let (native_word_res, gloss_word_res): (Vec<result::word::Item>, Vec<result::word::Item>) = futures::try_join!(
        search_word_by_native(db, query),
        search_word_by_glosses(db, query)
    )?;

    // Chain native and word results into one vector
    let word_results = native_word_res
        .into_iter()
        .chain(gloss_word_res)
        .collect_vec();

    // Chain and map results into one result vector
    let results = load_word_kanji_info(db, &word_results)
        .await?
        .into_iter()
        .map(|i| i.into())
        .collect::<Vec<result::Item>>()
        .into_iter()
        .chain(word_results.into_iter().map(|i| i.into()).collect_vec())
        .collect_vec();

    println!("full search took {:?}", start.elapsed());

    Ok(results)
}

/// Perform a native word search
pub async fn search_word_by_native(
    db: &DbPool,
    query: &str,
) -> Result<Vec<result::word::Item>, Error> {
    if !query.is_japanese() || query.is_empty() {
        return Ok(vec![]);
    }

    // Define basic search structure
    let mut word_search = word::WordSearch::new(db, query);
    word_search.with_language(Language::German);

    // Perform the word search
    let mut wordresults = if query.chars().count() <= 2 && query.is_kana() {
        // Search for exact matches only if query.len() <= 2
        word_search
            .with_mode(SearchMode::Exact)
            .search_native()
            .await?
    } else {
        word_search
            .with_mode(SearchMode::RightVariable)
            .search_native()
            .await?
    };

    // Sort the results based
    NativeWordOrder::new(query).sort(&mut wordresults);

    // Limit search to 10 results
    wordresults.truncate(10);

    Ok(wordresults)
}

/// load word assigned kanji
pub async fn load_word_kanji_info(
    db: &DbPool,
    words: &Vec<result::word::Item>,
) -> Result<Vec<kanji::Kanji>, Error> {
    use futures::future::join_all;

    let res: Vec<Vec<kanji::Kanji>> = join_all(
        words
            .iter()
            // Filter only words with kanji readings
            .filter_map(|i| {
                i.reading
                    .kanji
                    .is_some()
                    .then(|| i.reading.kanji.as_ref().unwrap())
            })
            // Don't load too much
            .take(10)
            // Load kanji from DB
            .map(|word| word.load_kanji_info(db)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<kanji::Kanji>>, Error>>()?;

    // if first word with kanji reading has more
    // than MAX_KANJI_INFO_ITEMS kanji, display all of them only
    let limit = {
        if !res.is_empty() && res[0].len() > MAX_KANJI_INFO_ITEMS {
            res[0].len()
        } else {
            MAX_KANJI_INFO_ITEMS
        }
    };

    // Remove duplicates
    let mut items_new = Vec::new();
    res.into_iter()
        .flatten()
        .collect_vec()
        .into_iter()
        .for_each(|i| {
            if !items_new.contains(&i) {
                items_new.push(i);
            }
        });

    // Limit result and map to result::Item
    Ok(items_new.into_iter().take(limit).collect_vec())
}

// TODO
/// Search gloss readings
pub async fn search_word_by_glosses(
    db: &DbPool,
    query: &str,
) -> Result<Vec<result::word::Item>, Error> {
    if query.is_japanese() {
        return Ok(vec![]);
    }

    let mut wordresults: Vec<result::word::Item> = Vec::new();

    let mut ws1 = word::WordSearch::new(db, query);
    ws1.with_language(Language::German)
        .with_mode(SearchMode::Exact);

    let mut ws2 = word::WordSearch::new(db, query);
    ws2.with_language(Language::German)
        .with_case_insensitivity(true)
        .with_mode(SearchMode::Exact);

    let (exact_words, mut case_ignoring) =
        futures::try_join!(ws1.search_by_glosses(), ws2.search_by_glosses())?;

    // TODO sort results

    /*
    exact_words.sort();
    case_ignoring.sort();
    */
    case_ignoring.retain(|i| !exact_words.contains(&i));

    // Search for exact matches

    /*
    let mut right_variable = word::WordSearch::new(db, query)
        .with_language(Language::German)
        .with_mode(SearchMode::RightVariable)
        .search_by_glosses()
        .await?;

    right_variable.retain(|i| !exact_words.contains(&i));
    right_variable.sort();
    right_variable.truncate(10);
    wordresults.extend(right_variable);
    */

    // Search for right variable
    wordresults.extend(exact_words);
    wordresults.extend(case_ignoring);

    Ok(wordresults)
}
