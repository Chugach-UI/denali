use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

pub fn parse_protocol(path: &Path) -> Protocol {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    quick_xml::de::from_reader::<_, Protocol>(reader).unwrap()
}

#[derive(Deserialize)]
pub struct Protocol {
    #[serde(rename = "@name")]
    name: String,
    copyright: Option<Copyright>,
    description: Option<Description>,
    #[serde(default, rename = "interface")]
    interfaces: Vec<Interface>,
}

#[derive(Deserialize)]
pub struct Copyright(String);

#[derive(Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@version")]
    version: String,
    description: Option<Description>,
    #[serde(rename = "$value")]
    elements: Vec<Element>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Element {
    Request(Request),
    Event(Event),
    Enum(Enum),
}

#[derive(Deserialize)]
pub struct Request {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    r#type: Option<String>,
    #[serde(rename = "@since")]
    since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    deprecated_since: Option<String>,
    description: Option<Description>,
    #[serde(default, rename = "arg")]
    args: Vec<Arg>,
}

#[derive(Deserialize)]
pub struct Event {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    r#type: Option<String>,
    #[serde(rename = "@since")]
    since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    deprecated_since: Option<String>,
    description: Option<Description>,
    #[serde(default, rename = "arg")]
    args: Vec<Arg>,
}

#[derive(Deserialize)]
pub struct Enum {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@since")]
    since: Option<String>,
    #[serde(rename = "@bitfield")]
    bitfield: Option<String>,
    description: Option<Description>,
    #[serde(default, rename = "entry")]
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct Entry {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@summary")]
    summary: Option<String>,
    #[serde(rename = "@since")]
    since: Option<String>,
    #[serde(rename = "@deprecated-since")]
    deprecated_since: Option<String>,
    description: Option<Description>,
}

#[derive(Deserialize)]
pub struct Arg {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "@summary")]
    summary: Option<String>,
    #[serde(rename = "@interface")]
    interface: Option<String>,
    #[serde(rename = "@allow-null")]
    allow_null: Option<String>,
    #[serde(rename = "@enum")]
    r#enum: Option<String>,
    description: Option<Description>,
}

#[derive(Deserialize)]
pub struct Description {
    #[serde(rename = "@summary")]
    summary: String,
    #[serde(rename = "$text")]
    content: Option<String>,
}
