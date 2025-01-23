use test_log::test;
use std::{cell::RefCell, rc::Rc};

use stream_protocol_lib::{json_stream_parser::{JsonStreamParser, ParserEvent}, ref_index_generator::RefIndexGenerator};

#[test]
fn test_unit() {
    let tests = [
        (
            r#""a string""#,
            r#"
2=""
2+="a string"
            "#.trim()
        ),
        (
            r#"true"#,
            r#"
2=true
            "#.trim()
        ),
        (
            r#" false "#,
            r#"
2=false
            "#.trim()
        ),
        (
            r#"[1,2]"#,
            r#"
2=[]
2+=1
2+=2
            "#.trim()
        ),
        (
            r#" [ 1 , 2 ] "#,
            r#"
2=[]
2+=1
2+=2
            "#.trim()
        ),
        (
            r#"["a","b"]"#,
            r#"
2=[]
2+="$ke$3"
3=""
3+="a"
2+="$ke$4"
4=""
4+="b"
            "#.trim()
        ),
        (
            r#"{
                "a": "b"
            }"#,
            r#"
2={}
2+={"a":"$ke$4"}
4=""
4+="b"
            "#.trim()
        ),
        (
            r#"{
                "a": {"nested": {"object": "is ok ?"}}
            }"#,
            r#"
2={}
2+={"a":"$ke$4"}
4={}
4+={"nested":"$ke$6"}
6={}
6+={"object":"$ke$8"}
8=""
8+="is ok ?"
            "#.trim()
        ),
        (
            r#"{
                "arr": [1,2]
            }"#,
            r#"
2={}
2+={"arr":"$ke$4"}
4=[]
4+=1
4+=2
            "#.trim()
        ),
        (
            r#"{
                "arr": ["a","b"]
            }"#,
            r#"
2={}
2+={"arr":"$ke$4"}
4=[]
4+="$ke$5"
5=""
5+="a"
4+="$ke$6"
6=""
6+="b"
            "#.trim()
        ),
        (
            r#"{
                "parent": {"child": "kid"}
            }"#,
            r#"
2={}
2+={"parent":"$ke$4"}
4={}
4+={"child":"$ke$6"}
6=""
6+="kid"
            "#.trim()
        ),
        (
            r#"[
                { "first_key": "first_val" },
                { "second_key": "second_val" }
            ]"#,
            r#"
2=[]
2+="$ke$3"
3={}
3+={"first_key":"$ke$5"}
5=""
5+="first_val"
2+="$ke$6"
6={}
6+={"second_key":"$ke$8"}
8=""
8+="second_val"
            "#.trim()
        ),
        (
            r#"[
                [ 1 , 2 , 3 ],
                ["a","b","c"]
            ]"#,
            r#"
2=[]
2+="$ke$3"
3=[]
3+=1
3+=2
3+=3
2+="$ke$7"
7=[]
7+="$ke$8"
8=""
8+="a"
7+="$ke$9"
9=""
9+="b"
7+="$ke$10"
10=""
10+="c"
            "#.trim()
        ),
        ( // There is a difference in processing logic with direct parent returns after numbers
            r#"[
                [1,2,3],
                ["a","b","c"]
            ]"#,
            r#"
2=[]
2+="$ke$3"
3=[]
3+=1
3+=2
3+=3
2+="$ke$7"
7=[]
7+="$ke$8"
8=""
8+="a"
7+="$ke$9"
9=""
9+="b"
7+="$ke$10"
10=""
10+="c"
            "#.trim()
        ),
        ( // There is a difference in processing logic with direct parent returns after numbers
            r#"[{
                "inner_arr": [{"inner_inner_obj": "val"}]
            }]"#,
            r#"
2=[]
2+="$ke$3"
3={}
3+={"inner_arr":"$ke$5"}
5=[]
5+="$ke$6"
6={}
6+={"inner_inner_obj":"$ke$8"}
8=""
8+="val"
            "#.trim()
        )
    ];

    for (idx, (input, expected_lines)) in tests.iter().enumerate() {
        println!("Running test {idx}...");
        let ref_index_generator = RefIndexGenerator::new();
        let buffer: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    
        ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
        let cnt = ref_index_generator.generate(); // 2
        
        let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, cnt);

        let expected_line_arr: Vec<&str> = expected_lines.split('\n').collect();
        
        let mut line_counter = 0;
        for byte in input.as_bytes() {
            let resp = json_stream_parser.add_char(byte);
            assert!(resp.is_ok());
        
            let mut buffer_b = buffer.borrow_mut();
            if buffer_b.len() > 0 {
                // Every non empty buffer is a line (or more) to test
                let output_rows = String::from_utf8(buffer_b.to_vec()).unwrap();
                // Sometimes output contains multiple rows
                let output_row_arr = output_rows.trim().split('\n');
                for output_row in output_row_arr {
                    assert_eq!(expected_line_arr[line_counter], output_row);
                    line_counter += 1;
                }
                buffer_b.clear(); // Simulate dumping the line
            }
            //println!("{} ==> {}", std::ascii::escape_default(*byte), String::from_utf8(buffer_b.to_vec()).unwrap());
        }
    }
}

#[test]
fn test_flush_regular() {
    let ref_index_generator = RefIndexGenerator::new();
    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2
    
    let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, cnt);
    
    let input = r#"{"key":"Some longer sentence"}"#;
    let expected_line_arr = [
        "2={}",
        r#"2+={"key":"$ke$4"}"#,
        "4=\"\"",
        "4+=\"Some \"",
        "4+=\"longer \"",
        "4+=\"sentence\"",
    ];

    let mut line_counter = 0usize;
    for byte in input.as_bytes() {
        let resp = json_stream_parser.add_char(byte);
        assert!(resp.is_ok());
        let mut output = resp.unwrap();

        if byte == &b' ' {
            assert!(output.is_none());
            output = json_stream_parser.flush();
        }
        if output.is_some() {
            // Every non empty buffer is a line (or more) to test
            let output_rows = output.unwrap();
            // Sometimes output contains multiple rows
            let output_row_arr = output_rows.trim().split('\n');
            for output_row in output_row_arr {
                assert_eq!(expected_line_arr[line_counter], output_row);
                line_counter += 1;
            }
        }
    }
}

#[test]
fn test_flush_inbetween_utf8_boundaries() {
    let ref_index_generator = RefIndexGenerator::new();

    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2
    
    let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, cnt);
    
    let input = r#""東京都飯田橋""#;
    let expected_line_arr = [
        r#"2="""#,
        r#"2+="東""#,
        r#"2+="京""#,
        r#"2+="都""#,
        r#"2+="飯""#,
        r#"2+="田""#,
        r#"2+="橋""#,
    ];

    let mut line_counter = 0usize;
    for byte in input.as_bytes() {
        let resp = json_stream_parser.add_char(byte);
        assert!(resp.is_ok());
        //println!("{} ==> {}", std::ascii::escape_default(*byte), String::from_utf8(buffer_b.to_vec()).unwrap());

        // First test the regular output
        let output = resp.unwrap();
        if output.is_some() {
            let output = output.unwrap();
            assert_eq!(expected_line_arr[line_counter], output.trim());
            line_counter += 1;
        }

        // Then, flush after every byte and test if we have the next line
        let output = json_stream_parser.flush();

        if output.is_some() {
            let output = output.unwrap();
            assert_eq!(expected_line_arr[line_counter], output.trim());
            line_counter += 1;
        }
    }
}

#[test]
fn test_flush_for_object_keys() {
    let ref_index_generator = RefIndexGenerator::new();

    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2
    
    let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, cnt);
    let inputs = [
        r#"{""#,
        r#"h"#,
        r#"1"#,
        r#"":""#,
        r#"da"#,
        r#"ta"#,
        r#""}"#,
    ];

    let reg_arr = [
        "2={}\n",
        "2+={\"h1\":\"$ke$4\"}\n4=\"\"\n",
    ];
    
    let flu_arr = [
        "4+=\"da\"\n",
        "4+=\"ta\"\n",
    ];

    let mut reg_i = 0;
    let mut flu_i = 0;
    for input in inputs {
        for byte in input.as_bytes() {
            if let Ok(Some(regular_output)) = json_stream_parser.add_char(&byte) {
                assert_eq!(regular_output.as_str(), reg_arr[reg_i]);
                reg_i += 1;
            }
        }
        if let Some(flushed_output) = json_stream_parser.flush() {
            assert_eq!(flushed_output.as_str(), flu_arr[flu_i]);
            flu_i += 1;
        }
    }
}

#[test]
fn test_gpt() {
    let ref_index_generator = RefIndexGenerator::new();

    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2

    let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, cnt)
        // Testing events
        .with_event_handler(ParserEvent::OnElementEnd, "references.0".to_string(), |value| {
            assert!(value.is_some());
            let value = value.unwrap(); 
            assert!(value.is_string());
            let value = value.as_str().unwrap();
            assert_eq!("source_1", value);
        })
        .with_event_handler(ParserEvent::OnElementEnd, "references.*".to_string(), |value| {
            assert!(value.is_some());
            let value = value.unwrap(); 
            assert!(value.is_string());
            let value = value.as_str().unwrap();
            assert!(value == "source_1" || value == "source_2"); // Any of the array due to the wildcard
        })
        .with_event_handler(ParserEvent::OnElementEnd, "test_escape".to_string(), |value| {
            assert!(value.is_some());
            let value = value.unwrap(); 
            assert!(value.is_string());
            let value = value.as_str().unwrap();
            assert_eq!(value, "line\ntab\tend");
        });
    let inputs = [
        r#"{"#,
        r#""references"#,
        r#"":[""#,
        r#"source"#,
        r#"_"#,
        r#"1"#,
        r#"",""#,
        r#"source"#,
        r#"_"#,
        r#"2"#,
        r#""],"#,
        r#""test_inner_json":"{\"inner_key\":\"inner_value\"}""#,
        r#""test_escape":"line\ntab\tend""#,
        r#""test_unicode":"\u2764\u2765""#,
        "}"
    ];

    let expected = [
        format!("{}\n", r#"2={}"#),
        format!("{}\n{}\n", r#"2+={"references":"$ke$4"}"#, "4=[]"), // Sometimes init happens immediately
        format!("{}\n{}\n", r#"4+="$ke$5""#, "5=\"\""),
        format!("{}\n", r#"5+="source""#),
        format!("{}\n", r#"5+="_""#),
        format!("{}\n", r#"5+="1""#),
        format!("{}\n{}\n", r#"4+="$ke$6""#, "6=\"\""),
        format!("{}\n", r#"6+="source""#),
        format!("{}\n", r#"6+="_""#),
        format!("{}\n", r#"6+="2""#),
        format!("{}\n{}\n", r#"2+={"test_inner_json":"$ke$8"}"#, "8=\"\""),
        format!("{}\n", r#"8+="{\"inner_key\":\"inner_value\"}""#),
        format!("{}\n{}\n", r#"2+={"test_escape":"$ke$10"}"#, "10=\"\""),
        format!("{}\n", r#"10+="line\ntab\tend""#), // These were escaped once, and re-JSONified for the protocol output
        format!("{}\n{}\n", r#"2+={"test_unicode":"$ke$12"}"#, "12=\"\""),
        format!("{}\n", r#"12+="❤❥""#), // Same here
    ];

    let mut i = 0;
    for input in &inputs {
        for byte in input.as_bytes() {
            if let Ok(Some(regular_output)) = json_stream_parser.add_char(&byte) {
                assert!(i < expected.len());
                assert_eq!(regular_output, expected[i]);
                i += 1;
            }
        }
        if let Some(flushed_output) = json_stream_parser.flush() {
            assert!(i < expected.len());
            assert_eq!(flushed_output, expected[i]);
            i += 1;
        }
    }
}