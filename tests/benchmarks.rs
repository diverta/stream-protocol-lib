// The compiler complains that crate level attributes need to belong in root module, but test feature is necessary here
#![allow(unused_attributes)]
#![feature(test)]
extern crate test;

use std::{fs, io::Read, rc::Rc};

use serde_json::Value;
use stream_protocol_lib::{json_stream_parser::{parser_options::ParserOptions, JsonStreamParser}, ref_index_generator::RefIndexGenerator};
use test::Bencher;

#[bench]
fn bench1(b: &mut Bencher) {
    let mut file = fs::File::open("tests/benchmarks/512kb.json").unwrap();
    let mut input = String::new();
    file.read_to_string(&mut input).unwrap();
    let input_json: Value = serde_json::from_str(&input).unwrap();
    b.iter(move || {
        let ref_index_generator = RefIndexGenerator::new();
        let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
            ref_index_generator,
            0,
            true,
            ParserOptions::default()
        );
        let mut c = 0;
        for byte in input.as_bytes() {
            let output = json_stream_parser.add_char(&byte);
            if let Err(output_err) = output {
                panic!("Error output at byte {}: {:?}", c, output_err)
            }
            c += 1;
        }
        let buffered_data = json_stream_parser.get_buffered_data();
        assert!(buffered_data.is_some());
        assert_eq!(buffered_data.unwrap(), &input_json);
    });
}