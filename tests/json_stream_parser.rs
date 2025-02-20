use serde_json::{json, Value};
use test_log::test;
use std::{cell::RefCell, rc::Rc, str::FromStr};

use stream_protocol_lib::{json_stream_parser::{JsonStreamParser, ParserEvent}, ref_index_generator::RefIndexGenerator};

#[test]
fn test_unit() {
    let tests = [
        (
            // Test 0
            r#""a string""#,
            r#"
2=""
2+="a string"
            "#.trim()
        ),
        (
            // Test 1
            r#"true"#,
            r#"
2=true
            "#.trim()
        ),
        (
            // Test 2
            r#" false "#,
            r#"
2=false
            "#.trim()
        ),
        (
            // Test 3
            r#"[1,2]"#,
            r#"
2=[]
2+=1
2+=2
            "#.trim()
        ),
        (
            // Test 4
            r#" [ 1 , 2 ] "#,
            r#"
2=[]
2+=1
2+=2
            "#.trim()
        ),
        (
            // Test 5
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
            // Test 6
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
            // Test 7
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
            // Test 8
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
            // Test 9
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
            // Test 10
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
            // Test 11
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
            // Test 12
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
        (
            // Test 13
            // There is a difference in processing logic with direct parent returns after numbers
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
        (
            // Test 14
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
        ),
        (
            // Test 15
            // Testing double up from array (due to it containing integers) into an object
            r#"{
                "grand_obj": {
                    "parent_arr": [1, 2],
                    "parent_single": "single_child"
                }
            }"#,
            r#"
2={}
2+={"grand_obj":"$ke$4"}
4={}
4+={"parent_arr":"$ke$6"}
6=[]
6+=1
6+=2
4+={"parent_single":"$ke$10"}
10=""
10+="single_child"
            "#.trim()
        )
    ];

    for (idx, (input, expected_lines)) in tests.iter().enumerate() {
        println!("Running test {idx}...");
        let ref_index_generator = RefIndexGenerator::new();
    
        ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
        let cnt = ref_index_generator.generate(); // 2
        
        let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
            ref_index_generator,
            cnt,
            true,
            None
        );

        let expected_line_arr: Vec<&str> = expected_lines
            .split('\n')
            .collect();
        
        let mut line_counter = 0;
        for byte in input.as_bytes() {
            let resp = json_stream_parser.add_char(byte);
            assert!(resp.is_ok());
            let resp = resp.unwrap();
        
            if let Some(out) = resp {
                // Sometimes output contains multiple rows
                let output_row_arr: std::str::Split<'_, char> = out.trim().split('\n');
                for output_row in output_row_arr {
                    assert_eq!(expected_line_arr[line_counter], output_row);
                    line_counter += 1;
                }
            }
        }
        
        // Testing buffered data
        let buffered_data = json_stream_parser.get_buffered_data();
        assert!(buffered_data.is_some());
        let buffered_data = buffered_data.unwrap();
        let input_value: Value = serde_json::from_str(input).unwrap();
        assert_eq!(&input_value, buffered_data);
    }
}

#[test]
fn test_flush_regular() {
    let ref_index_generator = RefIndexGenerator::new();
    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2
    
    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        cnt,
        false,
        None
    );
    
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
    
    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        cnt,
        false,
        None
    );
    
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
    
    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        cnt,
        false,
        None
    );
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

    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        cnt,
        false,
        None
    );

    // Testing events
    json_stream_parser.add_event_handler(ParserEvent::OnElementEnd, "references.0".to_string(), Box::new(|value: Option<Rc<Value>>| {
        assert!(value.is_some());
        let value = value.unwrap(); 
        assert!(value.is_string());
        let value = value.as_str().unwrap();
        assert_eq!("source_1", value);
    }));
    json_stream_parser.add_event_handler(ParserEvent::OnElementEnd, "references.*".to_string(), Box::new(|value: Option<Rc<Value>>| {
        assert!(value.is_some());
        let value = value.unwrap(); 
        assert!(value.is_string());
        let value = value.as_str().unwrap();
        assert!(value == "source_1" || value == "source_2"); // Any of the array due to the wildcard
    }));
    json_stream_parser.add_event_handler(ParserEvent::OnElementEnd, "test_escape".to_string(), Box::new(|value: Option<Rc<Value>>| {
        assert!(value.is_some());
        let value = value.unwrap(); 
        assert!(value.is_string());
        let value = value.as_str().unwrap();
        assert_eq!(value, "line\ntab\tend");
    }));
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

#[test]
fn test_gemini() {
    let input = r#"[{
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "a"
                },
                {
                  "text": "b"
                }
              ],
              "role": "model"
            }
          },
          {
            "content": {
              "parts": [
                {
                  "text": "c"
                },
                {
                  "text": "d"
                }
              ],
              "role": "model"
            }
          }
        ],
        "modelVersion": "gemini-2.0-flash-exp"
      },{
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "e"
                },
                {
                  "text": "f"
                }
              ],
              "role": "model"
            }
          },
          {
            "content": {
              "parts": [
                {
                  "text": "g"
                },
                {
                  "text": "h"
                }
              ],
              "role": "model"
            }
          }
        ],
        "modelVersion": "gemini-2.0-flash-exp"
      }
    ]"#;
    let ref_index_generator = RefIndexGenerator::new();

    // Expected items in reverse order
    let expected = RefCell::new(["h", "g", "f", "e", "d", "c", "b", "a"].to_vec());

    let mut json_stream_parser = JsonStreamParser::new(
        ref_index_generator,
        0,
        true,
        None
    )
        .with_event_handler(ParserEvent::OnElementEnd, "*.candidates.*.content.parts.*.text".to_string(), Box::new(move |value: Option<Rc<Value>>| {
            assert!(value.is_some());
            let value = value.unwrap(); 
            assert!(value.is_string());
            let value = value.as_str().unwrap();
            let results = &expected;
            let mut expected_b = results.borrow_mut();
            assert!(expected_b.len() > 0);
            let expected_item = expected_b.pop().unwrap();
            assert_eq!(value, expected_item);
        }));
    for byte in input.as_bytes() {
        assert!(json_stream_parser.add_char(&byte).is_ok());
    }

    // Testing buffered data
    let buffered_data = json_stream_parser.get_buffered_data();
    assert!(buffered_data.is_some());
    let buffered_data = buffered_data.unwrap();
    let input_value: Value = serde_json::from_str(input).unwrap();
    assert_eq!(&input_value, buffered_data);
}

#[test]
fn test_flush_buffer() {
    let input = r###"
{
  "answer": {
    "article": "## RAG（検索拡張生成）とは？その機能と活用\n\n**はじめに**\nRAG（Retrieval-Augmented Generation、検索拡張生成）は、大規模言語モデル（LLM）の能力を拡張する手法です。LLM>は膨大な知識を持っていますが、最新情報や特定のドメインに関する知識が不足している場合があります。RAGは、外部の知識ソースから関連情報を検索し、LLMに提供することで、より正確で最新の情報に
基づいた回答を生成できるようにします。\n\n## RAGの基本的な仕組み\nRAGは、大きく分けて「検索」と「生成」の2つのステップで構成されます。まず、ユーザーの質問やクエリに基づいて、関連する情
報を外部の知識ソースから検索します。次に、検索された情報と元の質問を組み合わせて、LLMが回答を生成します。これにより、LLMは自身の知識だけでなく、外部の情報も活用して、より質の高い回答を
提供できます。\n\n## RAGの主なメリット\nRAGの主なメリットは、LLMの知識を最新の状態に保ち、特定のドメインに関する知識を強化できることです。また、RAGは、LLMが回答の根拠となる情報を明示的
に示すことができるため、回答の信頼性を高めることができます。さらに、RAGは、LLMの学習データを更新することなく、新しい情報や知識を組み込むことができるため、LLMのメンテナンスコストを削減>できます。\n\n## RAGの様々な応用例\nRAGは、様々な分野で応用されています。例えば、カスタマーサポートの分野では、RAGを活用することで、顧客からの問い合わせに対して、より迅速かつ正確な回答
を提供できます。また、医療の分野では、RAGを活用することで、医師が最新の研究論文や臨床データに基づいて、より適切な診断や治療を行うことができます。さらに、金融の分野では、RAGを活用するこ
とで、アナリストが最新の市場動向や企業情報に基づいて、より正確な投資判断を行うことができます。\n\n## RAGとKurocoの連携\nKurocoはAPIファーストのヘッドレスCMSであり [[1](https://kuroco.app/)], [[2](https://kuroco.app/)], [[3](https://kuroco.app/)]、RAGとの連携が容易です。Kurocoに蓄積されたコンテンツをRAGの知識ソースとして活用することで、LLMはKurocoのコンテンツに基づい
て、より正確で関連性の高い回答を生成できます。例えば、Kurocoに製品マニュアルやFAQが登録されている場合、RAGを活用することで、顧客からの製品に関する問い合わせに対して、マニュアルやFAQの>情報に基づいて回答を生成できます。\n\n## RAGの今後の展望\nRAGは、LLMの能力を拡張するための重要な技術として、今後ますます発展していくと考えられます。今後は、より高度な検索技術や生成技術
が開発され、RAGの性能が向上していくことが期待されます。また、RAGは、様々な分野で応用され、私たちの生活や仕事に大きな影響を与える可能性があります。\n\n**結論**\nRAG（検索拡張生成）は、LLMの知識を拡張し、より正確で信頼性の高い回答を生成するための強力なツールです。様々な分野での応用が期待されており、今後の発展が楽しみです。\n\n![RAGの概念図](https://kuroco.app/ja/assets/images/image13-3fca67f6af405037b446436418391d4b.png)\n\n![Kurocoの機能](https://kuroco.app/files/user/img/top/img_function4.png)\n\n![Kurocoの比較](https://kuroco.app/files/user/img/top/img_compare1.png)",
    "title": "RAG（検索拡張生成）とは？仕組み、メリット、応用例を解説"
  }
}
"###;
    let input = input.replace("\n", ""); // Use spaces previously for readability, but remove now to make a valid JSON
    let ref_index_generator = RefIndexGenerator::new();

    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        0,
        true,
        None
    );
    let mut byte_counter = 0usize;
    for byte in input.as_bytes() {
        assert!(json_stream_parser.add_char(&byte).is_ok());
        if byte_counter != 0 && byte_counter % 40 == 0 {
            json_stream_parser.flush();
        }
        byte_counter += 1;
    }

    // Testing buffered data
    let buffered_data = json_stream_parser.get_buffered_data();
    assert!(buffered_data.is_some());
    let buffered_data = buffered_data.unwrap();
    let expected_data: Value = serde_json::from_str(&input).unwrap();
    assert_eq!(&expected_data, buffered_data);
}


#[test]
fn test_filters() {
    let input = r#"{
        "grand_obj": {
            "parent": {
                "child_arr": ["a", "b"],
                "child_str": "a string",
                "child_int": 123
            },
            "uncle_arr": [1, 2],
            "uncle_single": "single_child"
        }
    }"#;
    assert!(Value::from_str(input).is_ok());

    for (vec_whitelist, expected_lines, expected_buffer) in [
        (
            // Test 0 : no filters
            None,
            r#"
0={}
0+={"grand_obj":"$ke$2"}
2={}
2+={"parent":"$ke$4"}
4={}
4+={"child_arr":"$ke$6"}
6=[]
6+="$ke$7"
7=""
7+="a"
6+="$ke$8"
8=""
8+="b"
4+={"child_str":"$ke$10"}
10=""
10+="a string"
4+={"child_int":123}
2+={"uncle_arr":"$ke$14"}
14=[]
14+=1
14+=2
2+={"uncle_single":"$ke$18"}
18=""
18+="single_child"
            "#,
            json!({
                "grand_obj":{
                    "parent":{
                        "child_arr":["a","b"],
                        "child_str":"a string",
                        "child_int":{"child_int":123}
                    },
                    "uncle_arr":[1,2],
                    "uncle_single":"single_child"
                }
            })
        ),
        (
            // Test 1 : explicitly 0 filters
            Some(Vec::new()),
            r#"
0={}
            "#,
            json!({})
        ),
        (
            // Test 2
            Some(Vec::from([
                "grand_obj.parent".to_owned()
            ])),
            r#"
0={}
0+={"grand_obj":"$ke$2"}
2={}
2+={"parent":"$ke$4"}
4={}
4+={"child_arr":"$ke$6"}
6=[]
6+="$ke$7"
7=""
7+="a"
6+="$ke$8"
8=""
8+="b"
4+={"child_str":"$ke$10"}
10=""
10+="a string"
4+={"child_int":123}
"#,
            json!({
                "grand_obj":{
                    "parent":{
                        "child_arr":["a","b"],
                        "child_str":"a string",
                        "child_int":{"child_int":123}
                    }
                }
            })
        ),
        (
            // Test 3
            Some(Vec::from([
                "grand_obj.uncle_arr".to_owned(),
                "grand_obj.uncle_single".to_owned(),
            ])),
            r#"
0={}
0+={"grand_obj":"$ke$2"}
2={}
2+={"uncle_arr":"$ke$14"}
14=[]
14+=1
14+=2
2+={"uncle_single":"$ke$18"}
18=""
18+="single_child"
            "#,
            json!({
                "grand_obj":{
                    "uncle_arr":[1,2],
                    "uncle_single":"single_child"
                }
            })
        )
    ] {
        let ref_index_generator = RefIndexGenerator::new();
        let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
            ref_index_generator,
            0,
            true,
            vec_whitelist
        );
        let mut line_counter = 0;
        let expected_line_arr: Vec<&str> = expected_lines
            .trim()
            .split('\n')
            .collect();
        for byte in input.as_bytes() {
            let resp = json_stream_parser.add_char(byte);
            assert!(resp.is_ok());
            let resp = resp.unwrap();
        
            if let Some(out) = resp {
                // Sometimes output contains multiple rows
                let output_row_arr: std::str::Split<'_, char> = out.trim().split('\n');
                for output_row in output_row_arr {
                    assert_eq!(expected_line_arr[line_counter], output_row);
                    line_counter += 1;
                }
            }
        }
        
        // Testing buffered data
        let buffered_data = json_stream_parser.get_buffered_data();
        assert!(buffered_data.is_some());
        let buffered_data = buffered_data.unwrap();
        assert_eq!(&expected_buffer, buffered_data);
    }
}

#[test]
fn test_sample() {
    let input = r##"{
        "errors":[],
        "messages":[],
        "query":"サポート体制はどのようになっていますか？",
        "simplified_query":"サポート体制はどのようになっていますか？",
        "args":{"proper_nouns":"None","categories":["Documents-JA2"],"optimized_query":"support system overview"},
        "fallback_args":{"vector_search":"support system overview"},
        "server_timings":[
            {"key":"AI_completions","cached":true,"dur":20},{"key":"AI_optimize_query","cached":true,"dur":11},
            {"key":"AI_embeddings","cached":true,"dur":18},{"key":"AI_embeddings","cached":true,"dur":2}
        ],
        "contents_length": 94911,
        "list":[
            {
                "subject":"kuroco_infrastructure1.pdf",
                "slug":"kuroco-app-files-sheets-en-kuroco_infrastructure-pdf",
                "topics_group_id":8,
                "contents_type_nm":"Documents-JA2",
                "vector_distance":0.625,
                "contents":"contents"
            },
            {
                "subject":"kuroco_infrastructur2e.pdf",
                "slug":"kuroco-app-files-sheets-en-kuroco_infrastructure-pdf",
                "topics_group_id":8,
                "contents_type_nm":"Documents-JA2",
                "vector_distance":0.625,
                "contents":"contents"
            }
        ]
    }
    "##;

    let ref_index_generator = RefIndexGenerator::new();
    let mut json_stream_parser: JsonStreamParser<Box<dyn Fn(Option<Rc<Value>>)>> = JsonStreamParser::new(
        ref_index_generator,
        0,
        true,
        Some(Vec::from([
            "list.*.subject".to_owned()
        ]))
    );
    let mut line_counter = 0;
    for byte in input.as_bytes() {
        let resp = json_stream_parser.add_char(byte);
        assert!(resp.is_ok());
        let resp = resp.unwrap();
    
        if let Some(out) = resp {
            // Sometimes output contains multiple rows
            let output_row_arr: std::str::Split<'_, char> = out.trim().split('\n');
            for output_row in output_row_arr {
                //assert_eq!(expected_line_arr[line_counter], output_row);
                println!("{output_row}");
                line_counter += 1;
            }
        }
    }
    
    // Testing buffered data
    let buffered_data = json_stream_parser.get_buffered_data();
    assert!(buffered_data.is_some());
    let buffered_data = buffered_data.unwrap();
}
