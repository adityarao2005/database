#[derive(Debug, PartialEq, Clone)]
pub enum TupleElement {
    String(String),
    Integer(i64),
    Double(f64),
    Date {
        year: i32,
        month: i16,
        date: i16,
    },
    Time {
        hours: i16,
        minutes: i16,
        seconds: i16,
    },
    Boolean(bool),
    Null,
    Unknown,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tuple {
    pub elements: Vec<TupleElement>,
}
