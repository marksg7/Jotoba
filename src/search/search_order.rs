use crate::utils;

use super::query::Query;

#[cfg(feature = "tokenizer")]
use super::word::jp_parsing::WordItem;

#[cfg(feature = "tokenizer")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOrder<'a, 'parser> {
    pub query: &'a Query,
    pub morpheme: &'a Option<WordItem<'parser, 'a>>,
}

#[cfg(not(feature = "tokenizer"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOrder<'a> {
    pub query: &'a Query,
}

#[cfg(feature = "tokenizer")]
impl<'a, 'parser> SearchOrder<'a, 'parser> {
    pub fn new(query: &'a Query, morpheme: &'a Option<WordItem<'parser, 'a>>) -> Self {
        SearchOrder { query, morpheme }
    }

    pub fn sort<U, T>(&self, vec: &mut Vec<U>, order_fn: T)
    where
        T: Fn(&U, &SearchOrder) -> usize,
    {
        vec.sort_by(|a, b| utils::invert_ordering(order_fn(a, &self).cmp(&order_fn(b, &self))))
    }
}

#[cfg(not(feature = "tokenizer"))]
impl<'a, 'parser> SearchOrder<'a> {
    pub fn new(query: &'a Query) -> Self {
        SearchOrder { query }
    }

    pub fn sort<U, T>(&self, vec: &mut Vec<U>, order_fn: T)
    where
        T: Fn(&U, &SearchOrder) -> usize,
    {
        vec.sort_by(|a, b| utils::invert_ordering(order_fn(a, &self).cmp(&order_fn(b, &self))))
    }
}
