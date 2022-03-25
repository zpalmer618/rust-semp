use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub job_limit: usize,
    pub chunk_size: usize,
    pub sleep_int: usize,
    pub max_iter: usize,
    pub atom_names: Vec<String>,
}

impl Config {
    pub fn load(filename: &str) -> Self {
        let contents = std::fs::read_to_string(filename)
            .expect("failed to load config file");
        toml::from_str(&contents).expect("failed to deserialize config file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_full() {
        let got = Config::load("test_files/test.toml");
        let want = Config {
            job_limit: 10000,
            chunk_size: 128,
            sleep_int: 5,
            max_iter: 5,
            atom_names: vec!["C", "C", "C", "H", "H"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        assert_eq!(got, want);
    }

    // TODO have to implement Deserialize myself if I want partial ones

    // #[test]
    // fn test_load_part() {
    //     let got = Config::load("test_files/part.toml");
    //     dbg!(got);
    // }
}
