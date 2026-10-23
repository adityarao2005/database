use std::collections::HashMap;

use sql_query_parser::action::SqlAction;
use sql_query_parser::ddl::{KeyConstraint, TupleEntryDataType, TupleSchemaEntry};
use sql_query_parser::dml::select::{
    GroupBy, JoinType, Order, OrderClause, OrderBy, SelectItem, SqlQuery, SubQuery, TableRef,
};
use sql_query_parser::expression::{
    AggregateValues, Comparison, Expression, InValue, Literal, Predicate,
};
use sql_query_parser::parse;

// --- Helper Functions for Clean AST Construction in Tests ---

fn col(name: &str) -> Expression {
    Expression::Variable {
        name: name.to_string(),
        table: None,
    }
}

fn table_col(table: &str, name: &str) -> Expression {
    Expression::Variable {
        name: name.to_string(),
        table: Some(table.to_string()),
    }
}

fn int_lit(val: i64) -> Expression {
    Expression::Constant(Literal::Integer(val))
}

fn float_lit(val: f64) -> Expression {
    Expression::Constant(Literal::Float(val))
}

fn str_lit(val: &str) -> Expression {
    Expression::Constant(Literal::String(val.to_string()))
}

fn bool_lit(val: bool) -> Expression {
    Expression::Constant(Literal::Boolean(val))
}

fn null_lit() -> Expression {
    Expression::Constant(Literal::Null)
}

fn tbl(name: &str) -> TableRef {
    TableRef::Table {
        name: name.to_string(),
        alias: None,
    }
}

fn tbl_alias(name: &str, alias: &str) -> TableRef {
    TableRef::Table {
        name: name.to_string(),
        alias: Some(alias.to_string()),
    }
}

fn item(expr: Expression) -> SelectItem {
    SelectItem { expr, alias: None }
}

fn item_alias(expr: Expression, alias: &str) -> SelectItem {
    SelectItem {
        expr,
        alias: Some(alias.to_string()),
    }
}

fn simple_select(select: Vec<SelectItem>, from: TableRef) -> SqlQuery {
    SqlQuery::Basic {
        select,
        distinct: false,
        from,
        where_clause: None,
        group_by: None,
        order: None,
    }
}

// =========================================================================
// 1. BASIC DDL TESTS
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_create_table_basic() {
    let sql = "CREATE TABLE users (id INT, name VARCHAR(255));";
    let expected = SqlAction::CreateTable {
        name: "users".to_string(),
        entries: vec![
            TupleSchemaEntry {
                entry: "id".to_string(),
                data_type: TupleEntryDataType::Int,
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "name".to_string(),
                data_type: TupleEntryDataType::VarChar(255),
                not_null: false,
                default: None,
            },
        ],
        key_constraints: vec![],
        if_exists: false,
        check: vec![],
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_create_table_if_not_exists() {
    let sql = "CREATE TABLE IF NOT EXISTS items (id INT);";
    let expected = SqlAction::CreateTable {
        name: "items".to_string(),
        entries: vec![TupleSchemaEntry {
            entry: "id".to_string(),
            data_type: TupleEntryDataType::Int,
            not_null: false,
            default: None,
        }],
        key_constraints: vec![],
        if_exists: true,
        check: vec![],
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_drop_table_basic() {
    let sql = "DROP TABLE users;";
    let expected = SqlAction::DropTable {
        name: "users".to_string(),
        if_exists: false,
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_drop_table_if_exists() {
    let sql = "DROP TABLE IF EXISTS users;";
    let expected = SqlAction::DropTable {
        name: "users".to_string(),
        if_exists: true,
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

// =========================================================================
// 2. ADVANCED DDL TESTS
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_create_table_all_data_types_and_defaults() {
    let sql = "CREATE TABLE records (
        code CHAR(8) NOT NULL,
        bio VARCHAR(500),
        rating FLOAT DEFAULT 0.0,
        age INT DEFAULT 18,
        birth_date DATE,
        created_at TIME,
        notes VARCHAR(255) DEFAULT NULL
    );";

    let expected = SqlAction::CreateTable {
        name: "records".to_string(),
        entries: vec![
            TupleSchemaEntry {
                entry: "code".to_string(),
                data_type: TupleEntryDataType::Char(8),
                not_null: true,
                default: None,
            },
            TupleSchemaEntry {
                entry: "bio".to_string(),
                data_type: TupleEntryDataType::VarChar(500),
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "rating".to_string(),
                data_type: TupleEntryDataType::Float,
                not_null: false,
                default: Some(Literal::Float(0.0)),
            },
            TupleSchemaEntry {
                entry: "age".to_string(),
                data_type: TupleEntryDataType::Int,
                not_null: false,
                default: Some(Literal::Integer(18)),
            },
            TupleSchemaEntry {
                entry: "birth_date".to_string(),
                data_type: TupleEntryDataType::Date,
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "created_at".to_string(),
                data_type: TupleEntryDataType::Time,
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "notes".to_string(),
                data_type: TupleEntryDataType::VarChar(255),
                not_null: false,
                default: Some(Literal::Null),
            },
        ],
        key_constraints: vec![],
        if_exists: false,
        check: vec![],
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_create_table_with_constraints_and_check() {
    let sql = "CREATE TABLE orders (
        order_id INT NOT NULL,
        customer_id INT,
        tracking_no VARCHAR(100),
        amount FLOAT,
        PRIMARY KEY (order_id),
        UNIQUE (tracking_no),
        FOREIGN KEY (customer_id) REFERENCES customers (id),
        CHECK (amount > 0.0)
    );";

    let expected = SqlAction::CreateTable {
        name: "orders".to_string(),
        entries: vec![
            TupleSchemaEntry {
                entry: "order_id".to_string(),
                data_type: TupleEntryDataType::Int,
                not_null: true,
                default: None,
            },
            TupleSchemaEntry {
                entry: "customer_id".to_string(),
                data_type: TupleEntryDataType::Int,
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "tracking_no".to_string(),
                data_type: TupleEntryDataType::VarChar(100),
                not_null: false,
                default: None,
            },
            TupleSchemaEntry {
                entry: "amount".to_string(),
                data_type: TupleEntryDataType::Float,
                not_null: false,
                default: None,
            },
        ],
        key_constraints: vec![
            KeyConstraint::PrimaryKey(vec!["order_id".to_string()]),
            KeyConstraint::Unique(vec!["tracking_no".to_string()]),
            KeyConstraint::ForeignKey {
                entry: vec!["customer_id".to_string()],
                referenced_table: "customers".to_string(),
                referenced_entries: vec!["id".to_string()],
            },
        ],
        if_exists: false,
        check: vec![Predicate::Compare(
            Comparison::GreaterThan,
            Box::new(col("amount")),
            Box::new(float_lit(0.0)),
        )],
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

// =========================================================================
// 3. BASIC SQL QUERIES (SELECT)
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_simple_columns() {
    let sql = "SELECT id, name FROM users;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(col("id")), item(col("name"))],
        tbl("users"),
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_null_literal() {
    let sql = "SELECT id, NULL AS middle_name FROM users;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(col("id")), item_alias(null_lit(), "middle_name")],
        tbl("users"),
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_distinct_with_aliases() {
    let sql = "SELECT DISTINCT u.id AS user_id, u.name FROM users AS u;";
    let expected = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![
            item_alias(table_col("u", "id"), "user_id"),
            item(table_col("u", "name")),
        ],
        distinct: true,
        from: tbl_alias("users", "u"),
        where_clause: None,
        group_by: None,
        order: None,
    }));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_arithmetic_expressions() {
    let sql = "SELECT price * 1.1 + 5 - -discount / 2 FROM products;";
    let expected_expr = Expression::Subtract(
        Box::new(Expression::Add(
            Box::new(Expression::Multiply(
                Box::new(col("price")),
                Box::new(float_lit(1.1)),
            )),
            Box::new(int_lit(5)),
        )),
        Box::new(Expression::Divide(
            Box::new(Expression::Negate(Box::new(col("discount")))),
            Box::new(int_lit(2)),
        )),
    );

    let expected = SqlAction::Select(Box::new(simple_select(vec![item(expected_expr)], tbl("products"))));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_with_complex_where() {
    let sql = "SELECT id FROM users WHERE age >= 18 AND status = 'active' OR NOT deleted = true;";
    let where_pred = Predicate::Or(
        Box::new(Predicate::And(
            Box::new(Predicate::Compare(
                Comparison::GreaterThanEquals,
                Box::new(col("age")),
                Box::new(int_lit(18)),
            )),
            Box::new(Predicate::Compare(
                Comparison::Equals,
                Box::new(col("status")),
                Box::new(str_lit("active")),
            )),
        )),
        Box::new(Predicate::Not(Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(col("deleted")),
            Box::new(bool_lit(true)),
        )))),
    );

    let expected = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(where_pred),
        group_by: None,
        order: None,
    }));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_where_null_between_like() {
    let sql = "SELECT id FROM items WHERE description IS NOT NULL AND price BETWEEN 10 AND 50 AND name LIKE 'A%';";
    let where_pred = Predicate::And(
        Box::new(Predicate::And(
            Box::new(Predicate::IsNotNull(Box::new(col("description")))),
            Box::new(Predicate::Between(
                Box::new(col("price")),
                Box::new(int_lit(10)),
                Box::new(int_lit(50)),
            )),
        )),
        Box::new(Predicate::Like(Box::new(col("name")), "A%".to_string())),
    );

    let expected = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("items"),
        where_clause: Some(where_pred),
        group_by: None,
        order: None,
    }));

    assert_eq!(parse(sql).unwrap(), expected);
}

// =========================================================================
// 4. ADVANCED SQL QUERIES (AGGREGATES, FUNCTIONS, GROUP BY, HAVING, ORDER BY)
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_aggregates_and_functions() {
    let sql = "SELECT COUNT(*), COUNT(DISTINCT id), SUM(amount), AVG(score), MIN(low), MAX(high), COALESCE(bonus, 0) FROM stats;";
    let expected_items = vec![
        item(Expression::Aggregate(AggregateValues::CountStar)),
        item(Expression::Aggregate(AggregateValues::Count {
            expression: Box::new(col("id")),
            distinct: true,
        })),
        item(Expression::Aggregate(AggregateValues::Sum {
            expression: Box::new(col("amount")),
            distinct: false,
        })),
        item(Expression::Aggregate(AggregateValues::Avg {
            expression: Box::new(col("score")),
            distinct: false,
        })),
        item(Expression::Aggregate(AggregateValues::Min(Box::new(col("low"))))),
        item(Expression::Aggregate(AggregateValues::Max(Box::new(col("high"))))),
        item(Expression::Function {
            name: "COALESCE".to_string(),
            args: vec![Box::new(col("bonus")), Box::new(int_lit(0))],
        }),
    ];

    let expected = SqlAction::Select(Box::new(simple_select(expected_items, tbl("stats"))));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_select_group_by_having_order_by() {
    let sql = "SELECT dept, COUNT(*) AS emp_count FROM employees GROUP BY dept HAVING COUNT(*) > 5 ORDER BY emp_count DESC, dept ASC;";
    let expected = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![
            item(col("dept")),
            item_alias(
                Expression::Aggregate(AggregateValues::CountStar),
                "emp_count",
            ),
        ],
        distinct: false,
        from: tbl("employees"),
        where_clause: None,
        group_by: Some(GroupBy {
            group_values: vec!["dept".to_string()],
            having_clause: Some(Predicate::Compare(
                Comparison::GreaterThan,
                Box::new(Expression::Aggregate(AggregateValues::CountStar)),
                Box::new(int_lit(5)),
            )),
        }),
        order: Some(OrderBy {
            values: vec![
                OrderClause {
                    alias: "emp_count".to_string(),
                    order: Order::Desc,
                },
                OrderClause {
                    alias: "dept".to_string(),
                    order: Order::Asc,
                },
            ],
        }),
    }));

    assert_eq!(parse(sql).unwrap(), expected);
}

// =========================================================================
// 5. JOINS
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_cross_join() {
    let sql = "SELECT a.x, b.y FROM a CROSS JOIN b;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(table_col("a", "x")), item(table_col("b", "y"))],
        TableRef::Join {
            relation: Box::new(tbl("a")),
            join_type: JoinType::CrossJoin(Box::new(tbl("b"))),
        },
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_implicit_cross_join_comma() {
    // SQL-89 comma syntax: FROM a, b is an implicit cross join
    let sql = "SELECT a.x, b.y FROM a, b;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(table_col("a", "x")), item(table_col("b", "y"))],
        TableRef::Join {
            relation: Box::new(tbl("a")),
            join_type: JoinType::CrossJoin(Box::new(tbl("b"))),
        },
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_implicit_cross_join_multiple_tables_with_where() {
    // Comma join of 3 tables with classic SQL-89 style WHERE join condition
    let sql = "SELECT * FROM a, b, c WHERE a.id = b.a_id AND b.id = c.b_id;";
    let where_pred = Predicate::And(
        Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(table_col("a", "id")),
            Box::new(table_col("b", "a_id")),
        )),
        Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(table_col("b", "id")),
            Box::new(table_col("c", "b_id")),
        )),
    );

    let expected = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("*"))],
        distinct: false,
        from: TableRef::Join {
            relation: Box::new(TableRef::Join {
                relation: Box::new(tbl("a")),
                join_type: JoinType::CrossJoin(Box::new(tbl("b"))),
            }),
            join_type: JoinType::CrossJoin(Box::new(tbl("c"))),
        },
        where_clause: Some(where_pred),
        group_by: None,
        order: None,
    }));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_inner_join() {
    let sql = "SELECT u.name, o.id FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(table_col("u", "name")), item(table_col("o", "id"))],
        TableRef::Join {
            relation: Box::new(tbl_alias("users", "u")),
            join_type: JoinType::InnerJoin {
                other: Box::new(tbl_alias("orders", "o")),
                condition: Predicate::Compare(
                    Comparison::Equals,
                    Box::new(table_col("u", "id")),
                    Box::new(table_col("o", "user_id")),
                ),
            },
        },
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_left_right_full_outer_joins() {
    let sql = "SELECT * FROM a LEFT JOIN b ON a.id = b.id;";
    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(col("*"))],
        TableRef::Join {
            relation: Box::new(tbl("a")),
            join_type: JoinType::LeftJoin {
                other: Box::new(tbl("b")),
                condition: Predicate::Compare(
                    Comparison::Equals,
                    Box::new(table_col("a", "id")),
                    Box::new(table_col("b", "id")),
                ),
            },
        },
    )));
    assert_eq!(parse(sql).unwrap(), expected);

    let sql_right = "SELECT * FROM a RIGHT JOIN b ON a.id = b.id;";
    let expected_right = SqlAction::Select(Box::new(simple_select(
        vec![item(col("*"))],
        TableRef::Join {
            relation: Box::new(tbl("a")),
            join_type: JoinType::RightJoin {
                other: Box::new(tbl("b")),
                condition: Predicate::Compare(
                    Comparison::Equals,
                    Box::new(table_col("a", "id")),
                    Box::new(table_col("b", "id")),
                ),
            },
        },
    )));
    assert_eq!(parse(sql_right).unwrap(), expected_right);

    let sql_full = "SELECT * FROM a FULL JOIN b ON a.id = b.id;";
    let expected_full = SqlAction::Select(Box::new(simple_select(
        vec![item(col("*"))],
        TableRef::Join {
            relation: Box::new(tbl("a")),
            join_type: JoinType::FullJoin {
                other: Box::new(tbl("b")),
                condition: Predicate::Compare(
                    Comparison::Equals,
                    Box::new(table_col("a", "id")),
                    Box::new(table_col("b", "id")),
                ),
            },
        },
    )));
    assert_eq!(parse(sql_full).unwrap(), expected_full);
}

// =========================================================================
// 6. SET OPERATIONS
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_set_operations() {
    let q1 = simple_select(vec![item(col("id"))], tbl("table_a"));
    let q2 = simple_select(vec![item(col("id"))], tbl("table_b"));

    // UNION
    let sql_union = "SELECT id FROM table_a UNION SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_union).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::Union(Box::new(q1.clone()), Box::new(q2.clone()))))
    );

    // UNION ALL
    let sql_union_all = "SELECT id FROM table_a UNION ALL SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_union_all).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::UnionAll(Box::new(q1.clone()), Box::new(q2.clone()))))
    );

    // INTERSECT
    let sql_intersect = "SELECT id FROM table_a INTERSECT SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_intersect).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::Intersect(Box::new(q1.clone()), Box::new(q2.clone()))))
    );

    // INTERSECT ALL
    let sql_intersect_all = "SELECT id FROM table_a INTERSECT ALL SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_intersect_all).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::IntersectAll(Box::new(q1.clone()), Box::new(q2.clone()))))
    );

    // EXCEPT
    let sql_except = "SELECT id FROM table_a EXCEPT SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_except).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::Except(Box::new(q1.clone()), Box::new(q2.clone()))))
    );

    // EXCEPT ALL
    let sql_except_all = "SELECT id FROM table_a EXCEPT ALL SELECT id FROM table_b;";
    assert_eq!(
        parse(sql_except_all).unwrap(),
        SqlAction::Select(Box::new(SqlQuery::ExceptAll(Box::new(q1), Box::new(q2))))
    );
}

// =========================================================================
// 7. SUBQUERIES
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_subquery_in_from_clause() {
    let sql = "SELECT sub.id FROM (SELECT id FROM users) AS sub;";
    let inner_query = simple_select(vec![item(col("id"))], tbl("users"));

    let expected = SqlAction::Select(Box::new(simple_select(
        vec![item(table_col("sub", "id"))],
        TableRef::SubQuery {
            query: Box::new(inner_query),
            alias: "sub".to_string(),
        },
    )));

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_subquery_in_where_in_list_and_query() {
    // IN (List)
    let sql_list = "SELECT id FROM users WHERE status_code IN (1, 2, 3);";
    let expected_list = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(Predicate::In(
            Box::new(col("status_code")),
            InValue::List(vec![
                Literal::Integer(1),
                Literal::Integer(2),
                Literal::Integer(3),
            ]),
        )),
        group_by: None,
        order: None,
    }));
    assert_eq!(parse(sql_list).unwrap(), expected_list);

    // IN (SubQuery)
    let sql_sub = "SELECT id FROM users WHERE dept_id IN (SELECT id FROM departments);";
    let inner_query = simple_select(vec![item(col("id"))], tbl("departments"));
    let expected_sub = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(Predicate::In(
            Box::new(col("dept_id")),
            InValue::Query(SubQuery {
                sub_query: Box::new(inner_query),
                imported_variables: vec![],
            }),
        )),
        group_by: None,
        order: None,
    }));
    assert_eq!(parse(sql_sub).unwrap(), expected_sub);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_subquery_exists_all_any() {
    let inner_query = simple_select(vec![item(col("1"))], tbl("orders"));
    let sub = SubQuery {
        sub_query: Box::new(inner_query),
        imported_variables: vec![],
    };

    // EXISTS
    let sql_exists = "SELECT id FROM users WHERE EXISTS (SELECT 1 FROM orders);";
    let expected_exists = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(Predicate::Exists(Box::new(sub.clone()))),
        group_by: None,
        order: None,
    }));
    assert_eq!(parse(sql_exists).unwrap(), expected_exists);

    // ALL
    let sql_all = "SELECT id FROM users WHERE salary > ALL (SELECT 1 FROM orders);";
    let expected_all = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(Predicate::All(Comparison::GreaterThan, Box::new(col("salary")), Box::new(sub.clone()))),
        group_by: None,
        order: None,
    }));
    assert_eq!(parse(sql_all).unwrap(), expected_all);

    // ANY
    let sql_any = "SELECT id FROM users WHERE salary = ANY (SELECT 1 FROM orders);";
    let expected_any = SqlAction::Select(Box::new(SqlQuery::Basic {
        select: vec![item(col("id"))],
        distinct: false,
        from: tbl("users"),
        where_clause: Some(Predicate::Any(Comparison::Equals, Box::new(col("salary")), Box::new(sub))),
        group_by: None,
        order: None,
    }));
    assert_eq!(parse(sql_any).unwrap(), expected_any);
}

// =========================================================================
// 8. INSERT, UPDATE, AND DELETE
// =========================================================================

#[test]
#[ignore = "parser implementation in progress"]
fn test_insert_values_single_and_multi_row() {
    let sql = "INSERT INTO users (id, name, active) VALUES (1, 'Alice', true), (2, 'Bob', NULL);";
    let expected = SqlAction::Insert {
        table: "users".to_string(),
        columns: vec!["id".to_string(), "name".to_string(), "active".to_string()],
        value: vec![
            vec![int_lit(1), str_lit("Alice"), bool_lit(true)],
            vec![int_lit(2), str_lit("Bob"), null_lit()],
        ],
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_insert_into_from_select() {
    let sql = "INSERT INTO archive_users (id, name) SELECT id, name FROM users;";
    let select_query = simple_select(
        vec![item(col("id")), item(col("name"))],
        tbl("users"),
    );
    let expected = SqlAction::InsertInto {
        table: "archive_users".to_string(),
        columns: vec!["id".to_string(), "name".to_string()],
        sql_query: Box::new(select_query),
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_update_statement() {
    let sql = "UPDATE users SET name = 'Charlie', age = 30 WHERE id = 1;";
    let mut value_map = HashMap::new();
    value_map.insert("name".to_string(), Box::new(str_lit("Charlie")));
    value_map.insert("age".to_string(), Box::new(int_lit(30)));

    let expected = SqlAction::Update {
        table: "users".to_string(),
        value_map,
        where_clause: Some(Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(col("id")),
            Box::new(int_lit(1)),
        ))),
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_update_statement_with_null() {
    let sql = "UPDATE users SET status = NULL WHERE id = 1;";
    let mut value_map = HashMap::new();
    value_map.insert("status".to_string(), Box::new(null_lit()));

    let expected = SqlAction::Update {
        table: "users".to_string(),
        value_map,
        where_clause: Some(Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(col("id")),
            Box::new(int_lit(1)),
        ))),
    };

    assert_eq!(parse(sql).unwrap(), expected);
}

#[test]
#[ignore = "parser implementation in progress"]
fn test_delete_statement_with_and_without_where() {
    let sql_where = "DELETE FROM users WHERE id = 42;";
    let expected_where = SqlAction::Delete {
        table: "users".to_string(),
        where_clause: Some(Box::new(Predicate::Compare(
            Comparison::Equals,
            Box::new(col("id")),
            Box::new(int_lit(42)),
        ))),
    };
    assert_eq!(parse(sql_where).unwrap(), expected_where);

    let sql_all = "DELETE FROM users;";
    let expected_all = SqlAction::Delete {
        table: "users".to_string(),
        where_clause: None,
    };
    assert_eq!(parse(sql_all).unwrap(), expected_all);
}
