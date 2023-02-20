use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

pub type Welcome = Vec<WelcomeElement>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WelcomeElement {
    pub kind: String,
    pub version: String,
    pub metadata: Metadata,
    pub spec: Spec,
}

impl Display for WelcomeElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = self
            .metadata
            .labels
            .iter()
            .filter(|(k, _)| !k.starts_with("teleport.internal"))
            .map(|(k, v)| format!("{k}: {v}"))
            .join(", ");
        f.write_str(&display);
        Ok(())
    }
}

impl PartialEq for WelcomeElement {
    fn eq(&self, other: &Self) -> bool {
        self.metadata.name == other.metadata.name
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    pub name: String,
    pub labels: Labels,
    pub expires: String,
    pub id: f64,
}

type Labels = HashMap<String, String>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spec {
    pub addr: String,
    pub hostname: String,
    pub use_tunnel: Option<bool>,
    pub version: String,
    pub public_addr: Option<String>,
}