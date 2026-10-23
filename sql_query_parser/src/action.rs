use std::collections::HashMap;

use crate::{
    ddl::{KeyConstraint, TupleSchemaEntry},
    dml::select::SqlQuery,
    expression::{Expression, Predicate},
};

#[derive(Debug, PartialEq, Clone)]
pub enum SqlAction {
    CreateTable {
        name: String,
        entries: Vec<TupleSchemaEntry>,
        key_constraints: Vec<KeyConstraint>,
        if_exists: bool,
        check: Vec<Predicate>,
    },
    CreateView {
        name: String,
        material: bool,
        as_query: Box<SqlQuery>
    },
    DropView {
        name: String,
        if_exists: bool,
    },
    DropTable {
        name: String,
        if_exists: bool,
    },
    Select(Box<SqlQuery>),
    Insert {
        table: String,
        columns: Vec<String>,
        value: Vec<Vec<Expression>>,
    },
    InsertInto {
        table: String,
        columns: Vec<String>,
        sql_query: Box<SqlQuery>,
    },
    Update {
        table: String,
        value_map: HashMap<String, Box<Expression>>,
        where_clause: Option<Box<Predicate>>,
    },
    Delete {
        table: String,
        where_clause: Option<Box<Predicate>>,
    },
}
