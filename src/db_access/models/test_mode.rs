#[derive(Debug, Clone)]
pub struct TestMode {
    pub acronym: String,
    pub description: String,
}

impl From<String> for TestMode {
    fn from(value: String) -> Self {
        TestMode {
            acronym: value.to_string(),
            description: "".to_string(),
        }
    }
}
