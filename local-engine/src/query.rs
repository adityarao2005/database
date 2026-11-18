
pub enum QueryType {
    DQL,
    DDL,
    DML
}

// TODO: Implement a query parser and decoder for the local engine. This will involve parsing SQL queries and converting them into an internal representation that can be executed by the engine.
// This will use sqlparser-rs
pub fn decode_query(query: &str) -> Result<QueryType, String> {

    todo!("Implement query decoding logic for the local engine.");
}

pub fn optimize_predicate() {

    // TODO: Implement predicate optimization logic for queries.
}

pub fn optimize_query() {
    // TODO: implement query optimization logic for the select clauses.
    
}

