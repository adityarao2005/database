use std::collections::HashMap;

use crate::{
    ddl::{AlterTableAction, AlterViewAction, KeyConstraint, TupleSchemaEntry},
    dml::select::SqlQuery,
    expression::{Expression, Predicate},
};

#[derive(Debug, PartialEq, Clone)]
pub enum SqlAction {
    CreateIndex {
        name: String,
        table: String,
        columns: Vec<String>,
        unique: bool,
        if_not_exists: bool,
    },
    DropIndex {
        name: String,
        if_exists: bool,
    },
    AlterIndex {
        name: String,
        new_name: String,
    },
    AlterTable {
        name: String,
        action: AlterTableAction,
    },
    AlterView {
        name: String,
        action: AlterViewAction,
    },
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
        as_query: Box<SqlQuery>,
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
