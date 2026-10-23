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
    DropTable {
        name: String,
        if_exists: bool,
    },
    Select(SqlQuery),
    Insert {
        table: String,
        columns: Vec<String>,
        value: Vec<Vec<Expression>>,
    },
    InsertInto {
        table: String,
        columns: Vec<String>,
        sql_query: SqlQuery,
    },
    Update {
        table: String,
        value_map: HashMap<String, Expression>,
        where_clause: Option<Predicate>,
    },
    Delete {
        table: String,
        where_clause: Option<Predicate>,
    },
}
