use crate::teleport::{Host, Hosts};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
pub struct History {
    pub(crate) entries: Vec<Host>,
    path: PathBuf,
}

impl History {
    pub fn load(path: impl AsRef<Path>) -> Self {
        if !path.as_ref().exists() {
            History { path: path.as_ref().to_path_buf(), entries: Default::default() }.save();
        }
        let h = std::fs::File::open(path).expect("can't load history");
        serde_json::from_reader(h).expect("Error deserializing history")
    }

    pub fn update(mut self, host: &Host) -> Self {
        self.entries.retain(|x| x.metadata.name != host.metadata.name);
        self.entries.insert(0, host.to_owned());
        self.save();
        self
    }

    pub fn intersect(mut self, hosts: &Hosts) -> Self {
        self.entries.retain(|x| hosts.iter().any(|y| y.metadata.name == x.metadata.name));
        self.save();
        self
    }

    pub(crate) fn save(&self) {
        std::fs::write(&self.path, serde_json::to_string(self).unwrap()).unwrap();
    }
}
