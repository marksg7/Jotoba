use std::{
    fs::File,
    io::{BufReader, Write},
    path::Path,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
};

use deadpool_postgres::Pool;
use itertools::Itertools;
use models::kanji;
use parse::{
    kanjidict::{Character, Parser as kanjidictParser},
    parser::Parse,
};

const CHUNKSIZE: usize = 500;

/// Imports kanji dict into database
pub async fn import(db: &Pool, path: String) {
    println!("Clearing existing kanji");
    kanji::meaning::clear_meanings(db).await.unwrap();
    kanji::clear_kanji_elements(db).await.unwrap();
    kanji::clear_kanji(db).await.unwrap();

    let path = Path::new(&path);
    let parser = kanjidictParser::new(BufReader::new(File::open(path).unwrap()));

    let amount = kanjidictParser::new(BufReader::new(File::open(path).unwrap()))
        .count()
        .unwrap();

    let (sender, receiver): (SyncSender<Character>, Receiver<Character>) = sync_channel(CHUNKSIZE);
    let t1 = std::thread::spawn(move || {
        parser
            .parse(|entry, i| {
                if i % 100 == 0 {
                    print!("\rImporting kanjidict... {}%", i * 100 / amount);
                    std::io::stdout().flush().ok();
                }

                sender.send(entry).unwrap();

                false
            })
            .unwrap();
    });

    let mut rec_kanji: Vec<Character> = Vec::new();
    let mut received = receiver.recv();

    while received.is_ok() {
        rec_kanji.push(received.unwrap());

        if rec_kanji.len() + 400 > CHUNKSIZE {
            for kanji in rec_kanji.clone().into_iter().chunks(CHUNKSIZE).into_iter() {
                kanji::insert(db, kanji.collect_vec()).await.unwrap();
            }

            rec_kanji.clear();
        }
        received = receiver.recv();
    }

    // Insert rest
    kanji::insert(db, rec_kanji).await.unwrap();
    println!();

    t1.join().ok();
}
