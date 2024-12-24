use test_log::test;
use std::{cell::RefCell, pin::Pin, rc::Rc, string::FromUtf8Error, task::Poll};

use futures::{executor::block_on, AsyncWrite, AsyncWriteExt};
use stream_protocol_lib::{json_stream_parser::JsonStreamParser, ref_index_generator::RefIndexGenerator};

struct WriteSample {
    buf: Rc<RefCell<Vec<u8>>>
}

impl WriteSample {
    pub fn new(buf: Rc<RefCell<Vec<u8>>>) -> Self {
        Self {
            buf
        }
    }
    pub fn get_buffer(&self) -> Result<String, FromUtf8Error> {
        let buf = self.buf.borrow_mut();
        String::from_utf8(buf.to_vec())
    }
}

impl AsyncWrite for WriteSample {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut self_buf = self.buf.borrow_mut();
        self_buf.extend(buf);
        return Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

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
2+="a"
2+="b"
            "#.trim()
        ),
        (
            r#"{
                "a": "b"
            }"#,
            r#"
2={}
2+={"a":"b"}
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
6+={"object":"is ok ?"}
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
4+="a"
4+="b"
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
4+={"child":"kid"}
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
3+={"first_key":"first_val"}
2+="$ke$6"
6={}
6+={"second_key":"second_val"}
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
7+="a"
7+="b"
7+="c"
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
7+="a"
7+="b"
7+="c"
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
6+={"inner_inner_obj":"val"}
            "#.trim()
        )
    ];

    for (idx, (input, expected_lines)) in tests.iter().enumerate() {
        println!("Running test {idx}...");
        let ref_index_generator = RefIndexGenerator::new();
        let buffer: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let output = WriteSample::new(Rc::clone(&buffer));
    
        ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
        let cnt = ref_index_generator.generate(); // 2
        
        let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, output, cnt);

        let mut writer_wrapper = Pin::new(&mut json_stream_parser);
        let expected_line_arr: Vec<&str> = expected_lines.split('\n').collect();
        
        let mut line_counter = 0;
        for byte in input.as_bytes() {
            block_on(async {
                let resp = writer_wrapper.write(&[*byte]).await;
                assert!(resp.is_ok());
            });
        
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
fn test_flush() {
    let ref_index_generator = RefIndexGenerator::new();
    let buffer: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    let output = WriteSample::new(Rc::clone(&buffer));

    ref_index_generator.generate(); // 1 : generate once to simulate being in the middle
    let cnt = ref_index_generator.generate(); // 2
    
    let mut json_stream_parser = JsonStreamParser::new(ref_index_generator, output, cnt);

    let mut writer_wrapper = Pin::new(&mut json_stream_parser);
    
    let input = r#"{"key":"Some longer sentence"}"#;

    for byte in input.as_bytes() {
        block_on(async {
            let resp = writer_wrapper.write(&[*byte]).await;
            assert!(resp.is_ok());
        });

        let mut buffer_b = buffer.borrow_mut();
        let output_rows = String::from_utf8(buffer_b.to_vec()).unwrap();
        println!("{} ==> {}", std::ascii::escape_default(*byte), String::from_utf8(buffer_b.to_vec()).unwrap());

        let expected = [
            "4+=\"Some \"",
            "4+=\"longer \"",
            "4+=\"sentence\"",
        ];
        let mut counter = 0;
        if byte == &b' ' {
            assert!(buffer_b.len() == 0);
            block_on(async {
                assert!(writer_wrapper.flush().await.is_ok());
            });
            counter += 1;
        }
        buffer_b.clear(); // Simulate dumping the line
    }
}