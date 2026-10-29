use sql_query_parser::{ddl::{KeyConstraint, TupleSchemaEntry}, expression::Predicate};



#[derive(Debug, PartialEq, Clone)]
pub enum TupleElement {
    String(String),
    Integer(i64),
    Double(f64),
    Date {
        year: i32,
        month: i16,
        date: i16
    },
    Time {
        hours: i16,
        minutes: i16,
        seconds: i16
    },
    Boolean(bool),
    Null,
    Unknown
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tuple {
    pub elements: Vec<TupleElement>
}


pub struct Table {
    name: String,
    entries: Vec<TupleSchemaEntry>,
    check: Vec<Predicate>,
    key_constraint: Vec<KeyConstraint>
}

impl Table {
    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_columns(&self) -> std::slice::Iter<'_, TupleSchemaEntry> {
        self.entries.iter()
    }

    pub fn get_check_constraints(&self) -> std::slice::Iter<'_, Predicate> {
        self.check.iter()
    }

    pub fn get_key_constraints(&self) -> std::slice::Iter<'_, KeyConstraint> {
        self.key_constraint.iter()
    }
}