mod order;
pub mod result;
mod wordsearch;

use order::{GlossWordOrder, NativeWordOrder};
use result::{Item, Word};
pub use wordsearch::WordSearch;

use async_std::sync::Mutex;
use futures::future::try_join_all;
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::time::SystemTime;

use crate::{
    cache::SharedCache,
    error::Error,
    japanese::JapaneseExt,
    models::{dict::Dict, kanji},
    search::{
        query::{Query, QueryLang},
        SearchMode,
    },
    utils::real_string_len,
    DbPool,
};

use self::result::WordResult;

use super::{query::Form, utils};

/// An in memory Cache for word search results
static SEARCH_CACHE: Lazy<Mutex<SharedCache<u64, WordResult>>> =
    Lazy::new(|| Mutex::new(SharedCache::with_capacity(1000)));

const MAX_KANJI_INFO_ITEMS: usize = 5;

struct Search<'a> {
    db: &'a DbPool,
    query: &'a Query,
}

/// Search among all data based on the input query
pub async fn search(db: &DbPool, query: &Query) -> Result<WordResult, Error> {
    let start = SystemTime::now();
    let search = Search { query, db };

    // Try to use cache
    if let Some(c_res) = search.get_cache().await {
        return Ok(c_res);
    }

    // Do requested search
    let results = match query.form {
        Form::KanjiReading(_) => search.kanji_meaning_search().await,
        _ => search.do_word_search().await,
    }?;

    println!("search took {:?}", start.elapsed());
    search.save_cache(results.clone()).await;
    Ok(results)
}

impl<'a> Search<'a> {
    async fn kanji_meaning_search(&self) -> Result<WordResult, Error> {
        Ok(WordResult {
            items: vec![],
            contains_kanji: false,
        })
    }

    async fn do_word_search(&self) -> Result<WordResult, Error> {
        // Perform searches asynchronously
        let (native_word_res, gloss_word_res): (Vec<Word>, Vec<Word>) =
            futures::try_join!(self.native_results(), self.gloss_results())?;

        // Chain native and word results into one vector
        let word_results = native_word_res
            .into_iter()
            .chain(gloss_word_res)
            .collect_vec();

        // Chain and map results into one result vector
        let kanji_results = self.load_word_kanji_info(&word_results).await?;
        let kanji_items = kanji_results.len();

        let result = kanji_results
            .into_iter()
            .map(|i| i.into())
            .collect::<Vec<Item>>()
            .into_iter()
            .chain(word_results.into_iter().map(|i| i.into()).collect_vec())
            .collect_vec();

        return Ok(WordResult {
            items: result,
            contains_kanji: kanji_items > 0,
        });
    }

    /// Perform a native word search
    async fn native_results(&self) -> Result<Vec<Word>, Error> {
        if self.query.language != QueryLang::Japanese {
            return Ok(vec![]);
        }

        // Define basic search structure
        let mut word_search = WordSearch::new(self.db, &self.query.query);
        word_search
            .with_language(self.query.settings.user_lang)
            .with_english_glosses(self.query.settings.show_english);

        if self.query.has_part_of_speech_tags() {
            word_search.with_pos_filter(&self.query.get_part_of_speech_tags());
        }

        // Perform the word search

        let mut wordresults =
            if real_string_len(&self.query.query) <= 2 && self.query.query.is_kana() {
                // Search for exact matches only if query.len() <= 2
                let res = word_search
                    .with_mode(SearchMode::Exact)
                    .search_native()
                    .await?;

                if res.is_empty() {
                    // Do another search if no exact result was found
                    word_search
                        .with_mode(SearchMode::RightVariable)
                        .search_native()
                        .await?
                } else {
                    res
                }
            } else {
                word_search
                    .with_mode(SearchMode::RightVariable)
                    .search_native()
                    .await?
            };

        // Sort the results based
        NativeWordOrder::new(&self.query.query).sort(&mut wordresults);

        // Limit search to 10 results
        wordresults.truncate(10);

        Ok(wordresults)
    }

    /// Search gloss readings
    async fn gloss_results(&self) -> Result<Vec<Word>, Error> {
        if !(self.query.language == QueryLang::Foreign
            || self.query.language == QueryLang::Undetected)
        {
            return Ok(vec![]);
        }

        let mode = if real_string_len(&self.query.query) < 4 {
            SearchMode::Exact
        } else {
            SearchMode::Variable
        };

        // TODO don't make exact search
        let mut word_search = WordSearch::new(self.db, &self.query.query);
        word_search
            .with_language(self.query.settings.user_lang)
            .with_case_insensitivity(true)
            .with_english_glosses(self.query.settings.show_english)
            .with_mode(mode);

        if self.query.has_part_of_speech_tags() {
            word_search.with_pos_filter(&self.query.get_part_of_speech_tags());
        }

        let mut wordresults = word_search.search_by_glosses().await?;

        // Sort the results based
        GlossWordOrder::new(&self.query.query).sort(&mut wordresults);

        // Limit search to 10 results
        wordresults.truncate(10);

        Ok(wordresults)
    }

    /// Returns first 10 dicts of words which have a kanji
    fn get_kanji_words(words: &Vec<Word>) -> Vec<&Dict> {
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
            .collect_vec()
    }

    /// load word assigned kanji
    async fn load_word_kanji_info(&self, words: &Vec<Word>) -> Result<Vec<kanji::Kanji>, Error> {
        let kanji_words = Self::get_kanji_words(words);

        let retrieved_kanji = {
            // Also show kanji even if no word was found
            if !kanji_words.is_empty() {
                try_join_all(
                    kanji_words
                        .iter()
                        .map(|word| word.load_kanji_info(&self.db)),
                )
                .await?
                .into_iter()
                .flatten()
                .collect_vec()
            } else {
                // No words found, search only for kanji from query
                try_join_all(self.query.query.chars().into_iter().filter_map(|i| {
                    i.is_kanji()
                        .then(|| kanji::find_by_literal(&self.db, i.to_string()))
                }))
                .await?
            }
        };

        // If first word with kanji reading has more
        // than MAX_KANJI_INFO_ITEMS kanji, display all of them only
        let limit = {
            if !kanji_words.is_empty()
                && kanji_words[0].reading.kanji_count() > MAX_KANJI_INFO_ITEMS
            {
                kanji_words[0].reading.kanji_count()
            } else {
                MAX_KANJI_INFO_ITEMS
            }
        };

        // Limit result and map to result::Item
        Ok(utils::remove_dups(retrieved_kanji)
            .into_iter()
            .take(limit)
            .collect_vec())
    }
    async fn get_cache(&self) -> Option<WordResult> {
        SEARCH_CACHE
            .lock()
            .await
            .cache_get(&self.query.get_hash())
            .map(|i| i.clone())
    }

    async fn save_cache(&self, result: WordResult) {
        SEARCH_CACHE
            .lock()
            .await
            .cache_set(self.query.get_hash(), result);
    }
}