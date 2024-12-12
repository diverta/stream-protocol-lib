use std::{cell::RefCell, fmt::Debug, rc::Rc};

// Cheaply clonable struct that manages unique references
#[derive(Debug, PartialEq)]
pub struct RefIndexGenerator {
    internal_counter: Rc<RefCell<usize>>,
}

impl RefIndexGenerator {
    pub fn new() -> Self {
        Self {
            internal_counter: Rc::new(RefCell::new(0))
        }
    }
    pub fn generate(&self) -> usize {
        let mut counter = self.internal_counter.borrow_mut();
        *counter = *counter + 1;
        *counter
    }
}

impl Clone for RefIndexGenerator {
    fn clone(&self) -> Self {
        Self {
            internal_counter: Rc::clone(&self.internal_counter)
        }
    }
}