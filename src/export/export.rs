pub fn to_yaml<T: serde::Serialize>(file_path: &str, views: &T) {
    let value = serde_yaml::to_string(views).expect("Failed to serialize data to string");

    std::fs::write(file_path, value).expect("Failed to write yaml");
}