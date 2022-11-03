use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct DescribeInstances {
    #[serde(rename = "Reservations")]
    pub reservations: Vec<Reservation>,
}

#[derive(Serialize, Deserialize)]
pub struct Reservation {
    #[serde(rename = "Instances")]
    pub instances: Vec<Instance>,
}

#[derive(Serialize, Deserialize)]
pub struct Instance {
    #[serde(rename = "ImageId")]
    pub image_id: String,
    #[serde(rename = "InstanceId")]
    pub instance_id: String,
    #[serde(rename = "InstanceType")]
    pub instance_type: String,
    #[serde(rename = "KeyName")]
    pub key_name: Option<String>,
    #[serde(rename = "PrivateIpAddress")]
    pub private_ip_address: String,
    #[serde(rename = "PublicIpAddress")]
    pub public_ip_address: String,
    #[serde(rename = "State")]
    pub state: State,
    #[serde(rename = "Tags")]
    pub tags: Vec<Tag>,
    #[serde(rename = "PlatformDetails")]
    pub platform_details: String,
    #[serde(rename = "Platform")]
    pub platform: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "Code")]
    pub code: i64,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Tag {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "Value")]
    pub value: String,
}
