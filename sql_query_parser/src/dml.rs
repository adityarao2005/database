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

    pub enum SqlQuery {
        Basic {
            select: HashMap<String, Expression>,
            from: HashMap<String, TableLike>,
            where_clause: Predicate,
            group_by: GroupBy,
            order: OrderBy,
        },
        Union(Box<SqlQuery>, Box<SqlQuery>),
        UnionAll(Box<SqlQuery>, Box<SqlQuery>),
        Intersect(Box<SqlQuery>, Box<SqlQuery>),
        IntersectAll(Box<SqlQuery>, Box<SqlQuery>),
        Except(Box<SqlQuery>, Box<SqlQuery>),
        ExceptAll(Box<SqlQuery>, Box<SqlQuery>),
    }

    pub enum TableLike {
        Table(String),
        SubQuery(Box<SqlQuery>),
    }
}
