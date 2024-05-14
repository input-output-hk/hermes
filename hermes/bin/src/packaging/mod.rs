//! Hermes packaging.

#[cfg(test)]
mod tests {
    use hdf5::File;
    use temp_dir::TempDir;

    #[test]
    fn hdf5_dataset_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");
        let root_group = hdf5_file.as_group().expect("cannot create HDF5 group");

        println!(
            "hdf5 group: {root_group:?}, members: {:?}",
            root_group.member_names().expect("cannot get member names")
        );

        let metadata_json = "metadata.json";
        let metada_json_data = r#"{ "name": "Alex", "age": 25"}"#;
        root_group
            .new_dataset_builder()
            .with_data(metada_json_data)
            .create(metadata_json)
            .expect("cannot create metadata.json");

        println!(
            "hdf5 group: {root_group:?}, members: {:?}",
            root_group.member_names().expect("cannot get member names")
        );

        let metada_json = root_group
            .dataset(metadata_json)
            .expect("cannot open metadata.json");
        let data = String::from_utf8(metada_json.read_raw().expect("cannot read metadata.json"))
            .expect("cannot parse metadata.json");
        assert_eq!(data, metada_json_data);
    }
}
