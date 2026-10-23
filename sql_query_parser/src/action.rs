use std::collections::HashMap;

use crate::{
    dml::select::SqlQuery, ddl::{KeyConstraint, TupleSchemaEntry}, expression::Predicate,
};

pub enum SqlAction {
    CreateTable {
        name: String,
        entries: Vec<TupleSchemaEntry>,
        key_constraints: Vec<KeyConstraint>,
        if_exists: bool,
        check: Vec<Predicate>
    },
    DropTable {
        name: String,
        if_exists: bool,
    },
    Select(SqlQuery),
    Insert {
        table: String,
        value_map: HashMap<String, String>,
    },
    InsertInto {
        table: String,
        sql_query: SqlQuery,
    },
    Update {
        table: String,
        value_map: HashMap<String, String>,
        where_clause: Predicate,
    },
    Delete {
        table: String,
        where_clause: Predicate,
    },
}

