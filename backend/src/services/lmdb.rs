

mod tests {
    use arroy::Database;
    use arroy::distances::Euclidean;

    #[test]
    fn test_initialization() {
        /// That's the 200MiB size limit we allow LMDB to grow.
        const TWENTY_HUNDRED_MIB: usize = 2 * 1024 * 1024 * 1024;

        let dir = tempfile::tempdir().unwrap();
        let env = unsafe { heed::EnvOpenOptions::new().map_size(TWENTY_HUNDRED_MIB).open(dir.path()) }?;


        let mut wtxn = embedded_env.write_txn().unwrap();
        let nn_db: ArroyDatabase<Euclidean> = embedded_env.create_database(&mut wtxn, None).unwrap();
        let writer = Writer::<Euclidean>::new(nn_db, index, args.dimensions);

        let mut db_rw_txn = db_env.write_txn().unwrap();
        let db: HeedDatabase<U32<byteorder::NativeEndian>, SerdeBincode<FileInfo>> = db_env.create_database(&mut db_rw_txn, Some("serde-bincode")).expect("Error while creating the database");
        


        println!("initializing lmdb");
    }

}
