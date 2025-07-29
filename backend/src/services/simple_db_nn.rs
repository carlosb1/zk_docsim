use arroy::{Database as ArroyDatabase, Distance, ItemId, Reader, Writer};
use heed::types::{Bytes, U32};

use byteorder::BigEndian;
use heed::Database as HeedDatabase;
use heed::{Env, EnvOpenOptions};
use serde::{Deserialize, Serialize};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::fs;
use std::path::{Path, PathBuf};

type BEU32 = U32<BigEndian>;
const DEFAULT_DIMS: usize = 384;

const MAP_SIZE: usize = 1024 * 1024 * 1024 * 200;
const MAX_DBS: u32 = 100;
const NUM_THREADS: usize = 100;
const INDEX_DEFAULT_NN: u16 = 0;
const DEFAULT_SEED: u64 = 42;


pub trait Embeddable {
    fn to_embedding(&self, _: Vec<u8>) -> Vec<f32> {
        let values: [f32; DEFAULT_DIMS] = [0.; DEFAULT_DIMS];
        values.to_vec()
    }
}


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

pub struct SimpleDBNN<T: Embeddable, D: Distance> {
    pub env_db: Env,
    pub env_embedded: Env,
    pub nn_db: ArroyDatabase<D>,
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DBEntry {
    pub content: String,
    pub embedding: Vec<f32>,
}


#[derive(Debug, Clone)]
pub struct DBConfig<T> {
    pub db_path: PathBuf,
    pub embedded_path: PathBuf,
    pub config_path: PathBuf,
    pub embed_engine: T,
    pub dimensions: usize,
    pub index: u16,
    pub seed: u64,
}

impl<T: Default> Default for DBConfig<T> {
    fn default() -> Self {
        DBConfig {
            db_path: PathBuf::from("db"),
            embedded_path: PathBuf::from("embedded"),
            config_path: PathBuf::from("config"),
            embed_engine: T::default(),
            dimensions: DEFAULT_DIMS,
            index: 0,
            seed: 42,
        }
    }
}

impl<T: Default> DBConfig<T> {
    pub fn from_base_dir<P: AsRef<Path>>(base_dir: P) -> Self {
        let base = base_dir.as_ref();
        DBConfig {
            db_path: base.join("db"),
            embedded_path: base.join("embedded"),
            config_path: base.join("config"),
            embed_engine: T::default(),
            dimensions: DEFAULT_DIMS,
            index: 0,
            seed: 42,
        }
    }
}


impl<T: Default> DBConfig<T> {
    pub fn from_tempdir() -> Self {
        let tempdir = tempfile::tempdir().expect("failed to create tempdir");
        let base = tempdir.path();

        DBConfig {
            db_path: base.join("db"),
            embedded_path: base.join("embedded"),
            config_path: base.join("config"),
            embed_engine: T::default(),
            dimensions: DEFAULT_DIMS,
            index: INDEX_DEFAULT_NN,
            seed: DEFAULT_SEED,
        }
    }
}


impl<T: Embeddable, D: Distance> SimpleDBNN<T, D> {
    pub fn new(
        db_path: PathBuf,
        embedded_path: PathBuf,
        config_path: PathBuf,
        embed_engine: T,
        dimensions: usize,
        index: u16,
        seed: u64,
    ) -> anyhow::Result<Self> {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build_global();
        let _ = fs::create_dir_all(embedded_path.clone());
        let embedded = unsafe {
            EnvOpenOptions::new()
                .map_size(MAP_SIZE) // 2GiB
                .max_dbs(MAX_DBS)
                .open(embedded_path.clone())
        }?;
        let _ = fs::create_dir_all(db_path.clone());
        let db = unsafe {
            EnvOpenOptions::new()
                .map_size(MAP_SIZE) // 2GiB
                .max_dbs(MAX_DBS)
                .open(db_path.clone())
        }?;

        /* set up database for embedded */
        let mut embedded_wtxn = embedded.write_txn()?;
        let nn_db: ArroyDatabase<D> = embedded.create_database(&mut embedded_wtxn, None)?;
        embedded_wtxn.commit()?;

        /* heed db */
        let mut db_rw_txn = db.write_txn()?;
        let heed_db: HeedDatabase<BEU32, Bytes> =
            db.create_database(&mut db_rw_txn, Some("serde-bincode"))?;
        db_rw_txn.commit()?;

        let config = Config::load_config(config_path.to_str().expect("Could not load config path"))
            .unwrap_or_default();
        let rng = StdRng::seed_from_u64(seed);
        Ok(SimpleDBNN {
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
        })
    }

    pub fn from_config(config: DBConfig<T>) -> anyhow::Result<Self> {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build_global();

        let _ = fs::create_dir_all(&config.embedded_path);
        let embedded = unsafe {
            EnvOpenOptions::new()
                .map_size(MAP_SIZE)
                .max_dbs(MAX_DBS)
                .open(&config.embedded_path)
        }?;

        let _ = fs::create_dir_all(&config.db_path);
        let db = unsafe {
            EnvOpenOptions::new()
                .map_size(MAP_SIZE)
                .max_dbs(MAX_DBS)
                .open(&config.db_path)
        }?;

        let mut embedded_wtxn = embedded.write_txn()?;
        let nn_db: ArroyDatabase<D> = embedded.create_database(&mut embedded_wtxn, None)?;
        embedded_wtxn.commit()?;

        let mut db_rw_txn = db.write_txn()?;
        let heed_db: HeedDatabase<BEU32, Bytes> =
            db.create_database(&mut db_rw_txn, Some("serde-bincode"))?;
        db_rw_txn.commit()?;

        let loaded_config = Config::load_config(config.config_path.to_str().unwrap())
            .unwrap_or_default();
        let rng = StdRng::seed_from_u64(config.seed);

        Ok(SimpleDBNN {
            nn_db,
            heed_db,
            env_db: db,
            env_embedded: embedded,
            next_id: loaded_config.next_id,
            path_db: config.db_path,
            path_embedded: config.embedded_path,
            path_config: config.config_path,
            embed_engine: config.embed_engine,
            dimensions: config.dimensions,
            index: config.index,
            rng,
        })
    }

    pub fn get_current_id(self) -> u32 {
        self.next_id
    }
    pub fn update_id(&mut self, id: u32) {
        self.next_id = id;
    }

    pub fn nn_writer(&mut self, index: u16, dimensions: usize) -> Writer<D> {
        Writer::<D>::new(self.nn_db, index, dimensions)
    }



    fn put_db(&mut self, content: &str, id: u32, embedding: Vec<f32> ) -> anyhow::Result<()> {
        let mut txn = self.env_db.write_txn()?;
        let entry = DBEntry {
            content: String::from(content),
            embedding};

        let bytes = serde_json::to_vec(&entry)?;
        self.heed_db.put(&mut txn, &id, &bytes)?;
        txn.commit()?;
        Ok(())
    }

    fn put_batch_db(&mut self, batch: &Vec<(&str, u32, Vec<f32>)>) -> anyhow::Result<()> {
        let mut txn = self.env_db.write_txn()?;
        for (content, id, embedding) in batch {
            let db_entry = DBEntry{content: content.to_string(), embedding: embedding.to_vec()};
            self.heed_db.put(&mut txn, &id, serde_json::to_vec(&db_entry)?.as_ref())?;
        }
        txn.commit()?;
        Ok(())
    }

    fn get_db(&mut self, id: u32) -> anyhow::Result<Option<DBEntry>> {
        let rotxn = self.env_db.read_txn()?;
        let Ok(elem) = self.heed_db.get(&rotxn, &id) else {
            return Ok(None);
        };
        let Some(bytes) = elem else {
            return Ok(None);
        };

        let s = String::from_utf8_lossy(bytes);
        let entry: DBEntry = serde_json::from_slice(bytes)?;
        Ok(Some(entry))
    }

    fn put_nn(&mut self, content: &str, id: u32, index: u16) -> anyhow::Result<Vec<f32>> {
        let embedding = self.embed_engine.to_embedding(content.as_bytes().to_vec());
        let env = self.env_embedded.clone();
        let mut wtxn = env.write_txn()?;
        let writer = self.nn_writer(index, self.dimensions);
        writer.add_item(&mut wtxn, id, embedding.clone().as_slice())?;
        writer.builder(&mut self.rng).build(&mut wtxn)?;
        wtxn.commit()?;
        Ok(embedding)
    }

    fn put_batch_nn(&mut self, batch: &Vec<(&str, u32)>, index: u16) -> anyhow::Result<Vec<Vec<f32>>> {
        let env = self.env_embedded.clone();
        let mut wtxn = env.write_txn()?;
        let writer = self.nn_writer(index, self.dimensions);
        let mut embeds: Vec<Vec<f32>> = Vec::new();
        for (content, id) in batch {
            let embedding = self.embed_engine.to_embedding(content.as_bytes().to_vec());
            writer.add_item(&mut wtxn, *id, embedding.as_slice())?;
            embeds.push(embedding);
        }
        writer.builder(&mut self.rng).build(&mut wtxn)?;
        wtxn.commit()?;
        Ok(embeds)
    }


    fn get_nn(
        &mut self,
        content: &str,
        index: u16,
        n_results: usize,
    ) -> anyhow::Result<Vec<(u32, f32)>> {
        let embedding = self.embed_engine.to_embedding(content.as_bytes().to_vec());
        let rotxn = self.env_embedded.read_txn()?;
        let reader = Reader::<D>::open(&rotxn, index, self.nn_db)?;
        let query = reader.nns(n_results);
        let results = query.by_vector(&rotxn, embedding.as_slice())?;
        let ret_results = results
            .iter()
            .map(|&(itemid, near)| (itemid as u32, near))
            .collect::<Vec<(u32, f32)>>();
        Ok(ret_results)
    }


    pub fn put(&mut self, content: &str) -> anyhow::Result<Vec<f32>> {
        let current_id = self.next_id;
        let embedding = self.put_nn(content, current_id, self.index)?;
        self.put_db(content, current_id,  embedding.clone())?;
        self.next_id = self.next_id + 1;
        self.save_backup()?;
        Ok(embedding)
    }

    fn save_backup(&mut self) -> anyhow::Result<()> {
        Config::save_config(
            self.path_config
                .to_str()
                .expect("Could not save config path"),
            &Config {
                next_id: self.next_id,
            },
        )?;
        Ok(())
    }

    pub fn get(&mut self, content: &str, nn: usize) -> anyhow::Result<Vec<(u32, f32, DBEntry)>> {
        let nears = self.get_nn(content, self.index, nn)?;

        let results = nears
            .iter()
            .map(|&(index, dist)| {
                let val = self.get_db(index).unwrap().unwrap();
                (index, dist, val)
            })
            .collect::<Vec<(u32, f32, DBEntry)>>();
        Ok(results)
    }

    pub fn put_batch(&mut self, batch: Vec<&str>, index: u16) -> anyhow::Result<()> {
        let mut good_id_to_assign = self.next_id;
        let batch_with_indexes = batch
            .iter()
            .map(|&elem| {
                let result = (elem, good_id_to_assign);
                good_id_to_assign += 1;
                result
            })
            .collect::<Vec<(&str, u32)>>();
        let embeddings = self.put_batch_nn(batch_with_indexes.as_ref(), index)?;

        let batch_with_all: Vec<(&str, u32, Vec<f32>)> = batch_with_indexes.iter()
            .zip(embeddings.iter())
            .map(|((a, b), c)| (*a, *b, c.clone()))
            .collect();

        self.put_batch_db(batch_with_all.as_ref())?;
        self.next_id = good_id_to_assign;
        self.save_backup()?;
        Ok(())
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        let _ = fs::remove_dir_all(&self.path_db.clone());
        let _ = fs::remove_dir_all(&self.path_embedded.clone());
        let _ = fs::remove_dir_all(&self.path_config.clone());
        let _ = fs::remove_file(&self.path_config.clone());
        Ok(())
    }
}
pub fn remove(
    path_buf: &PathBuf,
    path_embedded: &PathBuf,
    path_config: &PathBuf,
) -> anyhow::Result<()> {
    let _ = fs::remove_dir_all(path_buf.clone());
    let _ = fs::remove_dir_all(path_embedded.clone());
    let _ = fs::remove_dir_all(path_config.clone());
    let _ = fs::remove_file(path_config.clone());
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::*;
    use arroy::distances::Euclidean;
    use fastembed::TextEmbedding;

    struct FastEmbeddingExample;

    impl Embeddable for FastEmbeddingExample {
        fn to_embedding(&self, content: Vec<u8>) -> Vec<f32> {
            let model =
                TextEmbedding::try_new(Default::default()).expect("It can not loaded the model");
            let formatted_content = format!("{:?}", content).clone();
            let documents = vec![formatted_content.as_str()];
            let embed = model.embed(documents, None).unwrap();
            embed.to_vec()[0].to_vec()
        }
    }

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
        let _ = remove(&db_path, &embedded_path, &config_path);

        let mut dummy_db: SimpleDBNN<DummyEmbedding, Euclidean> = SimpleDBNN::new(
            db_path.clone(),
            embedded_path.clone(),
            config_path.clone(),
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        )
            .unwrap();

        let content = "Hello, world!";
        dummy_db.put_db(content, 0,  vec![0.0; 10]).unwrap();

        let elems = dummy_db.get_db(0).unwrap().unwrap();
        let restored_content = elems.content;
        println!("{:?}", restored_content);
        assert_eq!(content, restored_content);
        let _ = remove(&db_path, &embedded_path, &config_path);
    }

    #[test]
    pub fn not_find_test() {
        let db_path = PathBuf::from("test_db2");
        let embedded_path = PathBuf::from("test_embedded_db2");
        let config_path = PathBuf::from("config2");
        let _ = remove(&db_path, &embedded_path, &config_path);
        let mut dummy_db: SimpleDBNN<DummyEmbedding, Euclidean> = SimpleDBNN::new(
            db_path.clone(),
            embedded_path.clone(),
            config_path.clone(),
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        )
            .unwrap();
        let content = "Hello, world!";
        dummy_db.put_db(content, 0, vec![0.0; 10]).unwrap();
        let non_found = dummy_db.get_db(1).unwrap();
        assert!(non_found.is_none());
        let _ = remove(&db_path, &embedded_path, &config_path);
    }

    #[test]
    pub fn nn_dummy_test() {
        let db_path = PathBuf::from("test_db");
        let embedded_path = PathBuf::from("test_embedded_db");
        let config_path = PathBuf::from("config");
        let _ = remove(&db_path, &embedded_path, &config_path);

        let mut dummy_db: SimpleDBNN<DummyEmbedding, Euclidean> = SimpleDBNN::new(
            db_path.clone(),
            embedded_path.clone(),
            config_path.clone(),
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        )
            .unwrap();

        let content = "Hello, world!";
        dummy_db.put_nn(content, 0, 0).unwrap();

        let content = "Hello, world2!";
        dummy_db.put_nn(content, 1, 0).unwrap();

        let content = "Hello, world3!";
        dummy_db.put_nn(content, 2, 0).unwrap();

        let content = "$$$$$$$$$$$";
        dummy_db.put_nn(content, 3, 0).unwrap();

        let results = dummy_db.get_nn("hello", 0, 4).unwrap();

        println!("{:?}", results);

        assert_eq!(4, results.len());
        let worse_result = results.last().unwrap();
        assert_eq!(worse_result.0, 3);
        assert!(worse_result.1 > 1000.0);
        let _ = remove(&db_path, &embedded_path, &config_path);
    }

    #[test]
    pub fn nn_batch_dummy_test() {
        let db_path = PathBuf::from("test_db");
        let embedded_path = PathBuf::from("test_embedded_db");
        let config_path = PathBuf::from("config");
        let _ = remove(&db_path.clone(), &embedded_path.clone(), &config_path.clone());

        let mut dummy_db: SimpleDBNN<DummyEmbedding, Euclidean> = SimpleDBNN::new(
            db_path.clone(),
            embedded_path.clone(),
            config_path.clone(),
            DummyEmbedding,
            DEFAULT_DIMS,
            0,
            46,
        )
            .unwrap();
        let _ = dummy_db.clear();

        let content1 = "Hello, world!";
        let content2 = "Hello, world2!";
        let content3 = "Hello, world3!";
        let content4 = "$$$$$$$$$$$";
        dummy_db
            .put_batch(vec![content1, content2, content3, content4], 0)
            .unwrap();

        let results = dummy_db.get_nn("hello", 0, 4).unwrap();

        println!("{:?}", results);

        assert_eq!(4, results.len());
        let worse_result = results.last().unwrap();
        assert!(worse_result.1 > 1000.0);
        let _ = remove(&db_path, &embedded_path, &config_path);
    }

    #[test]
    pub fn real_batch_dummy_test() {
        let db_path = PathBuf::from("test_dbb");
        let embedded_path = PathBuf::from("test_embedded_dbb");
        let config_path = PathBuf::from("configb");
        let _ = remove(&db_path, &embedded_path, &config_path);

        let mut dummy_db: SimpleDBNN<FastEmbeddingExample, Euclidean> = SimpleDBNN::new(
            db_path.clone(),
            embedded_path.clone(),
            config_path.clone(),
            FastEmbeddingExample,
            DEFAULT_DIMS,
            0,
            46,
        )
            .unwrap();

        let content1 = "Hello, world!";
        let content2 = "Hello, world2!";
        let content3 = "Hello, world3!";
        let content4 = "$$$$$$$$$$$";
        dummy_db
            .put_batch(vec![content1, content2, content3, content4], 0)
            .unwrap();

        let results = dummy_db.get("hello", 4).unwrap();


        println!("{:?}", results);
        assert_eq!(4, results.len());
        let worse_result = results.last().unwrap();
        assert_eq!(worse_result.0, 3);
        assert!(worse_result.1 > 0.5);
        let _ = remove(&db_path, &embedded_path, &config_path);
    }
}

