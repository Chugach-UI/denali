use std::fs::File;
use std::io::BufReader;

use serde::Deserialize;

pub fn parse_protocol(protocol: File) -> Result<Protocol, quick_xml::DeError> {
    let reader = BufReader::new(protocol);
    quick_xml::de::from_reader::<_, Protocol>(reader)
}

#[derive(Deserialize)]
pub struct Protocol {
    #[serde(rename = "@name")]
    pub name: String,
    pub copyright: Option<Copyright>,
    pub description: Option<Description>,
    #[serde(default, rename = "interface")]
    pub interfaces: Vec<Interface>,
}

#[derive(Deserialize)]
pub struct Copyright(String);

#[derive(Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub version: u32,
    pub description: Option<Description>,
    #[serde(rename = "$value")]
    pub elements: Vec<Element>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Request(Request),
    Event(Event),
    Enum(Enum),
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Request {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub type_: Option<String>,
    #[serde(rename = "@since")]
    pub since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    pub deprecated_since: Option<String>,
    pub description: Option<Description>,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub type_: Option<String>,
    #[serde(rename = "@since")]
    pub since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    pub deprecated_since: Option<String>,
    pub description: Option<Description>,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@since")]
    pub since: Option<String>,
    #[serde(rename = "@bitfield")]
    pub bitfield: Option<bool>,
    pub description: Option<Description>,
    #[serde(default, rename = "entry")]
    pub entries: Vec<Entry>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@summary")]
    pub summary: Option<String>,
    #[serde(rename = "@since")]
    pub since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    pub deprecated_since: Option<String>,
    pub description: Option<Description>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "@summary")]
    pub summary: Option<String>,
    #[serde(rename = "@interface")]
    pub interface: Option<String>,
    #[serde(rename = "@allow-null")]
    pub allow_null: Option<String>,
    #[serde(rename = "@enum")]
    pub enum_: Option<String>,
    pub description: Option<Description>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Description {
    #[serde(rename = "@summary")]
    pub summary: String,
    #[serde(rename = "$text")]
    pub content: Option<String>,
}
