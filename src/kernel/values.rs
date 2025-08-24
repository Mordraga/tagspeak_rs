// values.rs
#[derive(Clone, Debug)]
pub enum Value { Unit, Bool(bool), Num(f64), Str(String) }

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Num(n)  => Some(*n != 0.0 && !n.is_nan()),
            Value::Str(s)  => Some(!s.is_empty()),
            Value::Unit    => Some(false),
        }
    }
    pub fn try_num(&self) -> Option<f64> {
        match self { Value::Num(n) => Some(*n), Value::Str(s) => s.parse().ok(), _ => None }
    }
}
