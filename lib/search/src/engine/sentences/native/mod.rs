pub mod index;

use crate::engine::{document::SentenceDocument, simple_gen_doc::GenDoc, Indexable, SearchEngine};
use resources::{
    models::{sentences::Sentence, storage::ResourceStorage},
    parse::jmdict::languages::Language,
};
use vector_space_model::{DefaultMetadata, DocumentVector};

pub struct Engine {}

impl Indexable for Engine {
    type Metadata = DefaultMetadata;
    type Document = SentenceDocument;

    #[inline]
    fn get_index(
        _language: Option<Language>,
    ) -> Option<&'static vector_space_model::Index<Self::Document, Self::Metadata>> {
        Some(index::get())
    }
}

impl SearchEngine for Engine {
    type GenDoc = GenDoc;
    type Output = Sentence;

    #[inline]
    fn doc_to_output(
        storage: &'static ResourceStorage,
        inp: &Self::Document,
    ) -> Option<Vec<&'static Self::Output>> {
        storage.sentences().by_id(inp.seq_id).map(|i| vec![i])
    }

    fn gen_query_vector(
        index: &vector_space_model::Index<Self::Document, Self::Metadata>,
        query: &str,
    ) -> Option<DocumentVector<Self::GenDoc>> {
        let terms = tinysegmenter::tokenize(query);
        let query_document = GenDoc::new(terms);
        DocumentVector::new(index.get_indexer(), query_document.clone())
    }
}
