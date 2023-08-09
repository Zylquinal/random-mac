use std::fs;
use std::path::Path;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub trait MacInformation: erased_serde::Serialize {

    fn prefix(&self) -> String;

    fn vendor(&self) -> String;

    fn is_private(&self) -> bool;

    fn block_type(&self) -> String;

    fn random_from_prefix(&self) -> String {
        let mut rng = rand::thread_rng();
        let mut mac = self.prefix();
        for _ in 0..3 {
            mac.push_str(format!(":{:02X}", rng.gen_range(0..255)).as_str());
        }
        return mac;
    }

}

erased_serde::serialize_trait_object!(MacInformation);

trait MacData {

    fn convert(data: String) -> Result<Vec<Box<dyn MacInformation>>, String>;

}

#[derive(Serialize, Deserialize)]
pub struct DataSource {

    pub(crate) url: String,
    pub(crate) name: String,

}

impl DataSource {

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .expect(&*format!("Failed to read {:?}!", path));

        match serde_json::from_str(content.as_str()) {
            Ok(json) => Ok(json),
            Err(_) => return Err(String::from("Failed to parse JSON"))
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn fetch_information(&self) -> Result<Vec<Box<dyn MacInformation>>, String> {
        return fetch_information(self.clone());
    }

}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MacLookupApp {

    mac_prefix: String,
    vendor_name: String,
    private: bool,
    block_type: String,

}

impl MacInformation for MacLookupApp {

    fn prefix(&self) -> String {
        self.mac_prefix.clone()
    }

    fn vendor(&self) -> String {
        self.vendor_name.clone()
    }

    fn is_private(&self) -> bool {
        self.private
    }

    fn block_type(&self) -> String {
        self.block_type.clone()
    }

}

impl MacData for MacLookupApp {

    fn convert(data: String) -> Result<Vec<Box<(dyn MacInformation)>>, String> {
        let mut result: Vec<Box<dyn MacInformation>> = Vec::new();
        let json: Vec<MacLookupApp> = match serde_json::from_str(data.as_str()) {
            Ok(json) => json,
            Err(_) => return Err(String::from("Failed to parse JSON"))
        };
        for entry in json {
            result.push(Box::new(entry));
        }
        return Ok(result);
    }

}

fn fetch_information(data_source: &DataSource) -> Result<Vec<Box<dyn MacInformation>>, String> {
    let request = reqwest::blocking::get(data_source.url().as_str());
    let data = match request {
        Ok(response) => response.text(),
        Err(_) => return Err(String::from("Error fetching data"))
    };

    return match data {
        Ok(data) => convert(data_source.name(), data),
        Err(_) => Err(String::from("Error converting data"))
    };
}

pub fn convert(source_name: String, data: String) -> Result<Vec<Box<dyn MacInformation>>, String> {
    return match source_name.to_lowercase().as_str() {
        "maclookupapp" => MacLookupApp::convert(data),
        _ => Err(String::from("Invalid source name"))
    };
}

pub fn verify_prefix(prefix: &String) -> Result<(), String> {
    let prefix = prefix.replace(":", "");
    if prefix.len() != 6 {
        return Err(String::from("Invalid prefix length"));
    }
    for character in prefix.chars() {
        if !character.is_ascii_hexdigit() {
            return Err(String::from("Invalid prefix character"));
        }
    }
    return Ok(());
}