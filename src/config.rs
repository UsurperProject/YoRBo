use serde::{Serialize, Deserialize};
use uuid::Uuid;
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub partition: Option<(u32, u32)>,
    pub guid: Uuid
}

impl Default for Config {
    fn default() -> Self {
        Config { 
            partition: Some((0, 0)),
            guid: Uuid::nil()
         }
    }
}
