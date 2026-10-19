use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub partition: (u32, u32)
}

impl Default for Config {
    fn default() -> Self {
        Config { partition: (0, 0) }
    }
}
