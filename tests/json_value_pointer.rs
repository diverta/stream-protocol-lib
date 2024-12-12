use stream_protocol_lib::chunkers::json_value_pointer::JsonValuePointer;

#[test]
fn test_json_expr() {
    // Testing json expr
    let json_expr_to_test = [
        ("/", None),
        ("/a", None),
        ("/1", None),
        ("/b/c", Some("/b".to_string())),
        ("/b/c/0", Some("/b/c".to_string())),
    ];
    for (to_test, result) in json_expr_to_test {
        let pointer = JsonValuePointer {
            pointer_expr: Some(to_test.to_string())
        };
        assert_eq!(pointer.parent_expr(), result);
    }
}