use stream_protocol_lib::json_key_path::JsonKeyPath;

#[test]
fn test_json_key_path() {
    let mut json_key_path = JsonKeyPath::new();
    
    // Move up from root is false
    assert!(!json_key_path.move_up());

    assert!(json_key_path.move_down_object_or_array("parent_tmp"));
    assert!(json_key_path.match_expr("parent_tmp", false));
    assert!(!json_key_path.match_expr("", false));

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr("", false));
    assert!(!json_key_path.match_expr("parent", false));

    // Again check that moving up from root is false
    assert!(!json_key_path.move_up());

    assert!(json_key_path.move_down_object_or_array("1"));
    assert!(json_key_path.match_expr("1", false));
    assert!(json_key_path.match_expr("*", false));
    assert!(!json_key_path.match_expr("", false));

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr("", false));
    assert!(!json_key_path.match_expr("parent", false));

    assert!(json_key_path.move_down_object_or_array("parent"));
    assert!(json_key_path.match_expr("*", false));
    assert!(json_key_path.match_expr("parent", false));
    assert!(!json_key_path.match_expr("", false));

    assert!(json_key_path.move_down_object_or_array("0"));
    assert!(json_key_path.match_expr("parent.0", false));
    assert!(json_key_path.match_expr("parent.*", false));
    assert!(json_key_path.match_expr("*.0", false));
    assert!(json_key_path.match_expr("*.*", false));
    assert!(!json_key_path.match_expr("parent.1", false));
    assert!(!json_key_path.match_expr("parent", false));

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr("parent", false));
    assert!(json_key_path.match_expr("*", false));
    assert!(!json_key_path.match_expr("", false));
    assert!(!json_key_path.match_expr("parent.0", false));

    assert!(json_key_path.move_down_object_or_array("1"));
    assert!(json_key_path.match_expr("parent.1", false));
    assert!(json_key_path.match_expr("parent.*", false));
    assert!(!json_key_path.match_expr("parent.0", false));
    assert!(!json_key_path.match_expr("parent", false));

}

#[test]
fn test_json_key_path_gemini() {
    let mut json_key_path = JsonKeyPath::new();

    // Make path similar to Gemini stream
    assert!(json_key_path.move_down_object_or_array("0"));
    assert!(json_key_path.move_down_object_or_array("candidates"));
    assert!(json_key_path.move_down_object_or_array("0"));
    assert!(json_key_path.move_down_object_or_array("content"));
    assert!(json_key_path.move_down_object_or_array("parts"));
    assert!(json_key_path.move_down_object_or_array("0"));
    assert!(json_key_path.move_down_object_or_array("text"));

    assert!(json_key_path.match_expr("*.candidates.*.content.parts.*.text", false));
}