

pub mod select {
    use std::collections::HashMap;

    use crate::expression::{Expression, Predicate};

    pub struct GroupBy {
        pub group_values: Vec<String>,
        pub having_clause: Option<Predicate>,
    }

    pub enum Order {
        Asc,
        Desc,
    }

    pub struct OrderClause {
        pub alias: String,
        pub order: Order,
    }

    pub struct OrderBy {
        pub values: Vec<OrderClause>,
    }

    pub struct SqlQuery {
        pub select: HashMap<String, Expression>,
        pub from: HashMap<String, TableLike>,
        pub where_clause: Predicate,
        pub group_by: GroupBy,
        pub order: OrderBy,
    }

    pub enum TableLike {
        Table(String),
        SubQuery(Box<SqlQuery>),
    }
}