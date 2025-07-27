use arroy::distances::Euclidean;
use arroy::{Database as ArroyDatabase, Reader, Writer};
use heed::types::{Bytes, U32};

use byteorder::BigEndian;
use heed::Database as HeedDatabase;
use heed::{Env, EnvOpenOptions};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::fs;
use std::path::PathBuf;

type BEU32 = U32<BigEndian>;
const DEFAULT_DIMS: usize = 384;

pub trait Embeddable {
    fn to_embedding(&self, content: Vec<u8>) -> Vec<f32> {
        let values: [f32; DEFAULT_DIMS] = [0.; DEFAULT_DIMS];
        values.to_vec()
    }
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    next_id: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { next_id: 0 }
    }
}

impl Config {
    fn load_config(path: &str) -> std::io::Result<Config> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_config(path: &str, config: &Config) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(config)?;
        fs::write(path, content)
    }
}

pub struct DummyDB<T: Embeddable> {
    pub env_db: Env,
    pub env_embedded: Env,
    pub nn_db: ArroyDatabase<Euclidean>,
    pub heed_db: HeedDatabase<BEU32, Bytes>,
    pub next_id: u32,
    pub path_db: PathBuf,
    pub path_embedded: PathBuf,
    pub path_config: PathBuf,
    pub embed_engine: T,
    pub dimensions: usize,
    pub index: u16,
    pub rng: StdRng,
}

pub fn new_key(content: &str) -> [u8; 32] {
    let content_bytes = content.as_bytes();
    let bytes = blake3::hash(content_bytes);
    let hash_bytes: [u8; 32] = *bytes.as_bytes();
    hash_bytes
}

impl<T: Embeddable> DummyDB<T> {
    pub fn new(
        db_path: PathBuf,
        embedded_path: PathBuf,
        config_path: PathBuf,
        embed_engine: T,
        dimensions: usize,
        index: u16,
        seed: u64,
    ) -> Self {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(100)
            .build_global();
        let _ = fs::create_dir_all(embedded_path.clone());
        let embedded = unsafe {
            EnvOpenOptions::new()
                .map_size(1024 * 1024 * 1024 * 200) // 2GiB
                .max_dbs(100)
                .open(embedded_path.clone())
        }
        .unwrap();
        let _ = fs::create_dir_all(db_path.clone());
        let db = unsafe {
            EnvOpenOptions::new()
                .map_size(1024 * 1024 * 1024 * 200) // 2GiB
                .max_dbs(100)
                .open(db_path.clone())
        }
        .unwrap();

        /* set up database for embedded */
        let mut embedded_wtxn = embedded.write_txn().unwrap();
        let nn_db: ArroyDatabase<Euclidean> =
            embedded.create_database(&mut embedded_wtxn, None).unwrap();
        embedded_wtxn.commit().unwrap();

        /* heed db */
        let mut db_rw_txn = db.write_txn().unwrap();
        let heed_db: HeedDatabase<BEU32, Bytes> = db
            .create_database(&mut db_rw_txn, Some("serde-bincode"))
            .expect("Error while creating the database");
        db_rw_txn
            .commit()
            .expect("Error while committing the database");

        let config = Config::load_config(config_path.to_str().expect("Could not load config path"))
            .unwrap_or_default();
        let mut rng = StdRng::seed_from_u64(seed);
        DummyDB {
            nn_db,
            heed_db,
            env_db: db,
            env_embedded: embedded,
            next_id: config.next_id,
            path_db: db_path,
            path_embedded: embedded_path,
            path_config: config_path,
            embed_engine,
            dimensions,
            index,
            rng,
        }
    }

    pub fn get_current_id(self) -> u32 {
        self.next_id
    }
    pub fn update_id(&mut self, id: u32) {
        self.next_id = id;
    }

    pub fn nn_writer(&mut self, index: u16, dimensions: usize) -> Writer<Euclidean> {
        Writer::<Euclidean>::new(self.nn_db, index, dimensions)
    }

    fn put_db(&mut self, content: &str, id: u32) {
        let mut txn = self.env_db.write_txn().unwrap();
        self.heed_db.put(&mut txn, &id, content.as_bytes()).unwrap();

        txn.commit().unwrap();
    }

    fn get_db(&mut self, id: u32) -> Option<Vec<u8>> {
        let rotxn = self.env_db.read_txn().unwrap();
        let Ok(elem) = self.heed_db.get(&rotxn, &id) else {
            return None;
        };
        let Some(elem) = elem else {
            return None;
        };
        Some(elem.to_vec())
    }

    fn put_nn(&mut self, content: &str, id: u32, index: u16) {
        let embedding = self.embed_engine.to_embedding(content.as_bytes().to_vec());
        let env = self.env_embedded.clone();
        let mut wtxn = env.write_txn().unwrap();
        let writer = self.nn_writer(index, self.dimensions);
        writer
            .add_item(&mut wtxn, id, embedding.as_slice())
            .unwrap_or_default();
        writer.builder(&mut self.rng).build(&mut wtxn).unwrap();
        wtxn.commit().unwrap_or_default();
    }

    fn get_nn(
        &mut self,
        content: &str,
        index: u16,
        n_results: usize,
    ) -> anyhow::Result<Vec<(u32, f32)>> {
        let embedding = self.embed_engine.to_embedding(content.as_bytes().to_vec());
        let rotxn = self.env_embedded.read_txn().unwrap();
        let reader = Reader::<Euclidean>::open(&rotxn, index, self.nn_db).unwrap();
        let query = reader.nns(n_results);
        let results = query.by_vector(&rotxn, embedding.as_slice()).unwrap();
        let ret_results = results
            .iter()
            .map(|&(itemid, near)| (itemid as u32, near))
            .collect::<Vec<(u32, f32)>>();
        Ok(ret_results)
    }

    pub fn put(&mut self, content: &str) {
        let current_id = self.next_id;
        self.put_db(content, current_id);
        self.put_nn(content, current_id, self.index);
        self.next_id = self.next_id + 1;
        Config::save_config(
            self.path_config.to_str().unwrap(),
            &Config {
                next_id: self.next_id,
            },
        )
        .unwrap()
    }

    pub fn get(&mut self, content: &str, nn: usize) -> anyhow::Result<Vec<(u32, f32, String)>> {
        let possible_nears = self.get_nn(content, self.index, nn);
        let Ok(nears) = possible_nears else {
            return Ok(vec![]);
        };
        let results = nears
            .iter()
            .map(|&(index, dist)| {
                let val = self.get_db(index).unwrap();
                (index, dist, String::from_utf8(val).unwrap())
            })
            .collect::<Vec<(u32, f32, String)>>();
        Ok(results)
    }

    pub fn remove(&mut self) {
        fs::remove_dir_all(&self.path_db).unwrap();
        fs::remove_dir_all(&self.path_embedded).unwrap();
        fs::remove_dir_all(&self.path_config).unwrap();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct DummyEmbedding;

    impl Embeddable for DummyEmbedding {
        fn to_embedding(&self, content: Vec<u8>) -> Vec<f32> {
            let content_str = String::from_utf8(content).unwrap();
            let values: [f32; DEFAULT_DIMS] = if content_str.starts_with("$") {
                [100.; DEFAULT_DIMS]
            } else {
                [0.; DEFAULT_DIMS]
            };
            values.to_vec()
        }
    }
    #[test]
    pub fn dummy_test() {
        let db_path = PathBuf::from("test_db");
        let embedded_path = PathBuf::from("test_embedded_db");
        let config_path = PathBuf::from("config");

        let mut dummy_db = DummyDB::new(
            db_path,
            embedded_path,
            config_path,
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        );

        let content = "Hello, world!";
        dummy_db.put_db(content, 0);

        let elems = dummy_db.get_db(0).unwrap();
        let restored_content = String::from_utf8(elems.to_vec()).unwrap();
        println!("{:?}", restored_content);
        assert_eq!(content, restored_content);
    }

    #[test]
    pub fn not_find_test() {
        let db_path = PathBuf::from("test_db2");
        let embedded_path = PathBuf::from("test_embedded_db2");
        let config_path = PathBuf::from("config2");
        let mut dummy_db = DummyDB::new(
            db_path,
            embedded_path,
            config_path,
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        );
        let content = "Hello, world!";
        dummy_db.put_db(content, 0);
        let non_found = dummy_db.get_db(1);
        assert!(non_found.is_none());
    }

    #[test]
    pub fn nn_dummy_test() {
        let db_path = PathBuf::from("test_db");
        let embedded_path = PathBuf::from("test_embedded_db");
        let config_path = PathBuf::from("config");

        let mut dummy_db = DummyDB::new(
            db_path,
            embedded_path,
            config_path,
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        );

        let content = "Hello, world!";
        dummy_db.put_nn(content, 0, 0);

        let content = "Hello, world2!";
        dummy_db.put_nn(content, 1, 0);

        let content = "Hello, world3!";
        dummy_db.put_nn(content, 2, 0);

        let content = "$$$$$$$$$$$";
        dummy_db.put_nn(content, 3, 0);

        let results = dummy_db.get_nn("hello", 0, 4).unwrap();

        println!("{:?}", results);

        assert_eq!(4, results.len());
        let worse_result = results.last().unwrap();
        assert_eq!(worse_result.0, 3);
        assert!(worse_result.1 > 1000.0);
    }
}
