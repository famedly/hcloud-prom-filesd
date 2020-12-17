use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct FileSdEntry {
    pub targets: Vec<String>,
    pub labels: HashMap<String, String>,
}
