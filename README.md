# Jotoba
A free online, selfhostable, multilang japanese dictionary.

# Requirements
- [Jmdict.xml](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project)
- [Kanjidict2](https://www.edrdg.org/wiki/index.php/KANJIDIC_Project)
- [jmnedict.xml](http://www.edrdg.org/enamdict/enamdict_doc.html)
- [SVG files]()
- JLPT patches
- PostgresDB
- [Diesel](https://github.com/diesel-rs/diesel) with postgres feature (`cargo install diesel_cli --no-default-features --features postgres`)

### Optional
- [Audio files](https://github.com/tofugu/japanese-vocabulary-pronunciation-audio/tree/master/lib/ogg)
- Manga SFX
- Kanji stroke animations

# Installation
1. Setup a postgres DB
2. Customize and run `echo DATABASE_URL=postgres://username:password@localhost/jotoba > .env` 
3. Run `diesel setup && diesel migration run`
4. Compile it: `cargo build --release` (The binary will be located in ./target/release/jotoba)
5. Import kanji and jmdict: <br>
`jotoba -i --jmdict-path <path-to-jmdic> --kanjidict-path <path-to-kanjidict2>`
6. Start the server: 
`jotoba -s`
Joto-kun (including all of his variants) is licensed under [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/).
