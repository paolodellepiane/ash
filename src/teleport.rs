use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Welcome = Vec<WelcomeElement>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WelcomeElement {
    pub kind: String,
    pub version: String,
    pub metadata: Metadata,
    pub spec: Spec,
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
