use std::rc::Rc;
use json_node::JsonNode;
use json_value::JsonValue;

macro_rules! tri {
    ($e:expr $(,)?) => {
        match $e {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => return core::result::Result::Err(err),
        }
    };
}

pub(crate) mod json_node;
pub(crate) mod json_value;

pub(crate) struct JsonTree {
    pub root: Option<Rc<JsonNode>>,
}

impl JsonTree {
    pub fn new() -> Self {
        return Self {
            root: None
        }
    }

    pub fn set_root(&mut self, value: JsonValue) {
        self.root = Some(Rc::new(JsonNode::new(value)))
    }
}

#[test]
fn test() {
    use std::borrow::BorrowMut;
    use indexmap::IndexMap;

    let mut tree = JsonTree::new();
    tree.set_root(JsonValue::Object(IndexMap::new()));
    let root_node = tree.root.borrow_mut().as_ref().unwrap();
    {   
        assert!(Rc::clone(&root_node).add_to_object("arr_child".to_owned(), Rc::new(JsonNode::new(JsonValue::Array(vec![
            Rc::new(JsonNode::new(JsonValue::String("a".to_owned()))),
            Rc::new(JsonNode::new(JsonValue::Null)),
            Rc::new(JsonNode::new(JsonValue::Number(123.into()))),
        ])))).is_ok());
        assert!(Rc::clone(&root_node).add_to_object("bool_val".to_owned(), Rc::new(JsonNode::new(JsonValue::Boolean(true)))).is_ok());
        let root_value = root_node.value.borrow_mut();

        assert!(root_value.is_object());
        let root_val_obj = root_value.as_object().unwrap();

        assert!(root_val_obj.contains_key("arr_child"));
        let arr_child = root_val_obj.get("arr_child").unwrap().value.borrow();
        assert!(arr_child.is_array());
        let arr_child_arr = arr_child.as_array().unwrap();
        assert!(arr_child_arr.len() == 3);

        assert!(arr_child_arr.get(0).unwrap().value.borrow().is_string());
        assert!(arr_child_arr.get(1).unwrap().value.borrow().is_null());
        assert!(arr_child_arr.get(2).unwrap().value.borrow().is_number());

        assert!(root_val_obj.contains_key("bool_val"));
        assert!(root_val_obj.get("bool_val").unwrap().value.borrow().is_boolean());
    }

    let v = root_node.to_string();
    assert_eq!(v, r#"{"arr_child":["a",null,123],"bool_val":true}"#);
}