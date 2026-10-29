use std::{collections::HashMap, slice::Iter};

use sql_query_parser::{
    ddl::TupleSchemaEntry, dml::select::BasicSqlQuery, expression::{Expression, Predicate},
};

use crate::core::{Table, Tuple};

pub mod core;



pub trait Index {
    
    fn get_table_name() -> String;
    fn get_indexed_columns() -> Vec<TupleSchemaEntry>;   
}

pub struct TableManager {}

impl TableManager {
    pub async fn list_tables(&self) -> Vec<Table> {
        todo!("to be implemented")
    }

    pub async fn create_tables(&self, _table: Table) {
        todo!("to be implemented")
    }

    pub async fn insert_into_table(&self, _table: Table, _entries: Iter<'_, Tuple>) {
        todo!("to be implemented")
    }

    pub async fn delete_from_table(&self, _table: Table, _predicate: Predicate) {
        todo!("to be implemented")
    }

    pub async fn update_table(
        &self,
        _table: Table,
        _predicate: Predicate,
        _value_map: HashMap<String, Box<Expression>>,
    ) {
        todo!("to be implemented")
    }

    pub async fn select_tables(&self, _query: BasicSqlQuery) -> Iter<'_, Tuple> {
        todo!("to be implemented")
    }
}

