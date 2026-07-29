use caria_memory::CariaArc;

#[derive(Clone)]
pub enum CariaValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(CariaArc<String>),
    Object(CariaArc<Vec<CariaValue>>), // Simplified object representation
    Void,
}
