use std::rc::Rc;

#[derive(Debug)]
pub enum Type {
    Integer,
    Pointer(Rc<Type>),
}
