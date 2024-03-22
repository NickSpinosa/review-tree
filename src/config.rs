use dirs::home_dir;

pub struct Config {
    pub root_dir: String,
    pub num_threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: get_home_dir(),
            num_threads: 6,
        }
    }
}

fn get_home_dir() -> String {
    if let Some(path) = home_dir() {
        path.to_str().unwrap_or("/").to_owned()
    } else {
        "/".into()
    }
}
