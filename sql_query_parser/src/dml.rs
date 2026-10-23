pub mod select {

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

    pub struct SelectItem {
        pub expr: Expression,
        pub alias: Option<String>
    }

    pub enum SqlQuery {
        Basic {
            select: Vec<SelectItem>,
            distinct: bool,
            from: TableRef,
            where_clause: Option<Predicate>,
            group_by: Option<GroupBy>,
            order: Option<OrderBy>,
        },
        Union(Box<SqlQuery>, Box<SqlQuery>),
        UnionAll(Box<SqlQuery>, Box<SqlQuery>),
        Intersect(Box<SqlQuery>, Box<SqlQuery>),
        IntersectAll(Box<SqlQuery>, Box<SqlQuery>),
        Except(Box<SqlQuery>, Box<SqlQuery>),
        ExceptAll(Box<SqlQuery>, Box<SqlQuery>),
    }

    pub struct SubQuery {
        pub sub_query: Box<SqlQuery>,
        pub imported_variables: Vec<String>,
    }

    pub enum TableRef {
        Table {
            name: String,
            alias: Option<String>
        },
        SubQuery {
            query: Box<SqlQuery>,
            alias: String
        },
        Join {
            relation: Box<TableRef>,
            join_type: JoinType,
        },
    }

    pub enum JoinType {
        CrossJoin(Box<TableRef>),
        LeftJoin {
            other: Box<TableRef>,
            condition: Predicate,
        },
        RightJoin {
            other: Box<TableRef>,
            condition: Predicate,
        },
        InnerJoin {
            other: Box<TableRef>,
            condition: Predicate,
        },
        FullJoin {
            other: Box<TableRef>,
            condition: Predicate,
        },
    }
}
