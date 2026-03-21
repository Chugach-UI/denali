use std::fs::File;

fn main() {
    let protocol_path = std::env::args().nth(1).unwrap();

    let protocol_raw =
        denali_protocol_parser::parse_protocol(File::open(protocol_path).unwrap())
            .unwrap();
    let converted = toml::to_string_pretty(&protocol_raw).unwrap();
    std::fs::write("converted.toml", converted).unwrap();
}
