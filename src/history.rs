use crate::config::Config;
use crate::teleport::{Host, Hosts};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct History {
    pub(crate) entries: Vec<Host>,
}

impl History {
    pub fn load() -> Self {
        let path = Config::history_path();
        if !path.exists() {
            History { ..Default::default() }.save();
        }
        let h = std::fs::File::open(path).expect("can't load history");
        serde_json::from_reader(h).expect("Error deserializing history")
    }

    pub fn update(host: &Host) {
        let mut h = Self::load();
        h.entries.retain(|x| x.metadata.name != host.metadata.name);
        h.entries.insert(0, host.to_owned());
        h.save();
    }

    pub fn intersect(hosts: &Hosts) {
        let mut h = Self::load();
        h.entries.retain(|x| hosts.iter().any(|y| y.metadata.name == x.metadata.name));
        h.save();
    }

    pub(crate) fn save(&self) {
        std::fs::write(Config::history_path(), serde_json::to_string(self).unwrap()).unwrap();
    }
}
