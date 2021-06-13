mod binary_search;
mod jaro_search;
pub mod store_item;
pub mod text_store;

use std::collections::HashMap;

use binary_search::Search as BinarySearch;
use jaro_search::Search as JaroSearch;
use parse::jmdict::languages::Language;
use strsim::jaro_winkler;
use text_store::TextStore;

use self::store_item::Item;

#[derive(Clone)]
pub struct SuggestionSearch<T: TextStore> {
    dicts: HashMap<Language, TextSearch<T>>,
}

impl<T: TextStore> SuggestionSearch<T> {
    pub fn new(text_store: HashMap<Language, TextSearch<T>>) -> Self {
        Self { dicts: text_store }
    }

    /// Searches for suggestions in the provided language and uses english as fallback
    pub fn search<'a>(&'a self, query: &'a str, lang: Language) -> Option<Vec<&'a T::Item>> {
        if query.is_empty() {
            return None;
        }

        let mut res: Vec<&T::Item> = self.do_search(query, lang).unwrap_or_default();

        if res.len() < 5 {
            // Search for english
            res.extend(self.do_search(query, Language::English).unwrap_or_default());

            // Do jaro search
            if query.len() > 4 {
                println!("do jaro search: {}, {}", res.len(), query.len());
                // TODO this blocks sometimes very long. Make to future!!
                // let dict = self.dicts.get(&lang)?;
                // res.extend(dict.find_jaro(query, 4));
            }
        }

        // Order by best match against `query`
        res.sort_by(|l, r| {
            Self::result_order_value(r.get_text(), query)
                .cmp(&Self::result_order_value(l.get_text(), query))
        });

        // Remove duplicates
        res.dedup_by(|a, b| a.get_text() == b.get_text());

        Some(res)
    }

    fn result_order_value(query: &str, v: &str) -> u32 {
        (jaro_winkler(&v.get_text().to_lowercase(), query) * 100_f64) as u32
    }

    /// Searches for one language
    fn do_search<'a>(&'a self, query: &'a str, lang: Language) -> Option<Vec<&'a T::Item>> {
        let dict = self.dicts.get(&lang)?;

        let mut res: Vec<_> = dict.find_binary(query.to_owned()).take(100).collect();

        // Also search for 1st one with uppercase
        if query.chars().next().unwrap().is_lowercase() {
            res.extend(dict.find_binary(utils::first_letter_upper(query)).take(100));
        }

        Some(res)
    }
}

#[derive(Clone, Copy)]
pub struct TextSearch<T: TextStore> {
    text_store: T,
}

impl<T: TextStore> TextSearch<T> {
    /// Creates a new [`Serach`] based on searchable data. The input must be sorted and implement
    /// `Ord`
    pub fn new(text_store: T) -> Self {
        Self { text_store }
    }

    /// Returns a vector over all found elements
    pub fn find_all_bin<'a>(&'a self, query: String) -> Vec<&'a T::Item> {
        if query.is_empty() {
            return vec![];
        }

        BinarySearch::new(query, &self.text_store)
            .search()
            .collect()
    }

    /// Returns a vector over all found elements
    pub fn find_all_lev<'a>(&'a self, query: &'a str, len_limit: usize) -> Vec<&'a T::Item> {
        if query.is_empty() {
            return vec![];
        }

        self.find_jaro(query, len_limit).collect()
    }

    pub fn find_jaro<'a>(
        &'a self,
        query: &'a str,
        len_limit: usize,
    ) -> impl Iterator<Item = &'a T::Item> {
        self.jaro_search(query, len_limit).search()
    }

    pub fn find_binary<'a>(&'a self, query: String) -> impl Iterator<Item = &'a T::Item> {
        self.binary_search(query).search()
    }

    fn binary_search<'a>(&'a self, query: String) -> BinarySearch<'a, T> {
        BinarySearch::new(query, &self.text_store)
    }

    fn jaro_search<'a>(&'a self, query: &'a str, len_limit: usize) -> JaroSearch<'a, T> {
        JaroSearch::new(query, &self.text_store, len_limit)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufRead, BufReader, Write},
        time::SystemTime,
    };

    use strsim::jaro_winkler;

    use super::*;

    const TS: &'static [&'static str] = &["a", "abc", "add", "b", "bbc"];
    const TS2: &'static [&'static str] = &["b", "bbc"];
    const B_TS: &'static [&'static str] = &["a", "b", "go", "golang", "rust"];

    fn simple_ts() -> TextSearch<'static, &'static [&'static str]> {
        TextSearch::new(&TS)
    }

    fn simple_ts2() -> TextSearch<'static, &'static [&'static str]> {
        TextSearch::new(&TS2)
    }

    fn bigger_ts() -> TextSearch<'static, &'static [&'static str]> {
        TextSearch::new(&B_TS)
    }

    fn simple_dataset() -> Vec<TextSearch<'static, &'static [&'static str]>> {
        vec![simple_ts(), simple_ts2()]
    }

    #[test]
    fn first_matches() {
        for search in simple_dataset() {
            let e = search.find_all_bin("b");
            assert_eq!(e, vec![&"b", &"bbc"]);
        }
    }

    #[test]
    fn one_element_store() {
        let data = vec!["b"];
        let search = TextSearch::new(&data);
        let e = search.find_all_bin("b");
        assert_eq!(e, vec![&"b"]);
    }

    #[test]
    fn empty_query() {
        for search in simple_dataset() {
            let e = search.find_all_bin("");
            let empty: Vec<&&str> = Vec::new();
            assert_eq!(e, empty);
        }
    }

    #[test]
    fn not_found() {
        for search in simple_dataset() {
            let e = search.find_all_bin("0");
            let empty: Vec<&&str> = Vec::new();
            assert_eq!(e, empty);
        }
    }

    #[test]
    fn test_2st_matches() {
        for search in simple_dataset() {
            let e = search.find_all_bin("bb");
            assert_eq!(e, vec![&"bbc"]);
        }
    }

    #[test]
    fn one_element_store_not_found() {
        let data = vec!["b"];
        let search = TextSearch::new(&data);
        let e = search.find_all_bin("0");
        let empty: Vec<&&str> = Vec::new();
        assert_eq!(e, empty);
    }

    #[test]
    fn test_matches_complex() {
        let search = bigger_ts();
        let e = search.find_all_bin("go");
        assert_eq!(e, vec![&"go", &"golang"]);
    }

    #[test]
    fn test_matches_complex2() {
        let search = bigger_ts();
        let e = search.find_all_bin("ga");
        let empty: Vec<&&str> = Vec::new();
        assert_eq!(e, empty);
    }

    #[test]
    fn file() {
        let file = File::open("./output_en.json").unwrap();
        let reader = BufReader::new(file);
        let vec: Vec<String> = reader.lines().map(|i| i.unwrap()).collect();

        let searc = TextSearch::new(&vec);
        let start = SystemTime::now();
        let res = searc.find_all_bin("music");
        println!("binary found: {}", res.len());
        println!("binary took {:?}", start.elapsed());
    }

    #[test]
    fn file_lev() {
        let file = File::open("./output_en.json").unwrap();
        let reader = BufReader::new(file);
        let vec: Vec<String> = reader.lines().map(|i| i.unwrap()).collect();

        let searc = TextSearch::new(&vec);
        let start = SystemTime::now();
        let query = "cmon";
        let mut res: Vec<&String> = searc.find_jaro(query, 5).collect();
        res.sort_by(|l, r| {
            let l_j = (jaro_winkler(l, query) * 100_f64) as u32;
            let r_j = (jaro_winkler(r, query) * 100_f64) as u32;
            r_j.cmp(&l_j)
        });

        println!("{:#?}", res.iter().take(10000).collect::<Vec<_>>());
        println!("lev found: {}", res.len());
        println!("lev took {:?}", start.elapsed());
    }
}