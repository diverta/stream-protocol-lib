use stream_protocol_lib::json_key_path::JsonKeyPath;


#[test]
fn test_json_key_path() {
    let mut json_key_path = JsonKeyPath::new();
    
    // Move up from root is false
    assert!(!json_key_path.move_up());

    assert!(json_key_path.move_down_object_or_array("parent_tmp"));
    assert!(json_key_path.match_expr("parent_tmp"));
    assert!(!json_key_path.match_expr(""));

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr(""));
    assert!(!json_key_path.match_expr("parent"));

    // Again check that moving up from root is false
    assert!(!json_key_path.move_up());

    assert!(json_key_path.move_down_object_or_array("1"));
    assert!(json_key_path.match_expr("1"));
    assert!(json_key_path.match_expr("*"));
    assert!(!json_key_path.match_expr(""));

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr(""));
    assert!(!json_key_path.match_expr("parent"));

    assert!(json_key_path.move_down_object_or_array("parent"));
    assert!(json_key_path.match_expr("*"));
    assert!(json_key_path.match_expr("parent"));
    assert!(!json_key_path.match_expr(""));

    assert!(json_key_path.move_down_object_or_array("0"));
    assert!(json_key_path.match_expr("parent.0"));
    assert!(json_key_path.match_expr("parent.*"));
    assert!(json_key_path.match_expr("*.0"));
    assert!(!json_key_path.match_expr("parent.1"));
    assert!(!json_key_path.match_expr("parent"));
    assert!(!json_key_path.match_expr("*.*")); // Double wildcard is NG

    assert!(json_key_path.move_up());
    assert!(json_key_path.match_expr("parent"));
    assert!(json_key_path.match_expr("*"));
    assert!(!json_key_path.match_expr(""));
    assert!(!json_key_path.match_expr("parent.0"));

    assert!(json_key_path.move_down_object_or_array("1"));
    assert!(json_key_path.match_expr("parent.1"));
    assert!(json_key_path.match_expr("parent.*"));
    assert!(!json_key_path.match_expr("parent.0"));
    assert!(!json_key_path.match_expr("parent"));

}