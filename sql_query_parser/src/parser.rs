use std::collections::HashMap;

use crate::action::SqlAction;
use crate::ddl::{AlterTableAction, AlterColumnAction, AlterViewAction, KeyConstraint, TupleEntryDataType, TupleSchemaEntry};
use crate::dml::select::{
    BasicSqlQuery, GroupBy, JoinType, Order, OrderBy, OrderClause, SelectItem, SqlQuery, SubQuery, TableRef,
};
use crate::error::ParseError;
use crate::expression::{
    AggregateValues, Comparison, Expression, InValue, Literal, Predicate,
};
use crate::lexer::{tokenize, SpannedToken, Token};

pub fn parse(input: &str) -> Result<SqlAction, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let action = parser.parse_statement()?;
    parser.consume(&Token::Semicolon);
    if !parser.is_at_end() {
        let pos = parser.current_pos();
        return Err(ParseError::with_position(
            format!("Unexpected token after statement: {:?}", parser.peek()),
            pos,
        ));
    }
    Ok(action)
}

struct Parser {
    tokens: Vec<SpannedToken>,
    cursor: usize,
    parameter_count: usize,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            cursor: 0,
            parameter_count: 0,
        }
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|t| &t.token)
    }


    fn current_pos(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map(|t| t.start)
            .unwrap_or_else(|| {
                self.tokens
                    .last()
                    .map(|t| t.end)
                    .unwrap_or(0)
            })
    }

    fn next(&mut self) -> Option<&SpannedToken> {
        if self.is_at_end() {
            None
        } else {
            let tok = &self.tokens[self.cursor];
            self.cursor += 1;
            Some(tok)
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.peek() == Some(token)
    }

    fn consume(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token, desc: &str) -> Result<SpannedToken, ParseError> {
        if self.check(token) {
            Ok(self.next().unwrap().clone())
        } else {
            let pos = self.current_pos();
            Err(ParseError::with_position(
                format!("Expected {}, found {:?}", desc, self.peek()),
                pos,
            ))
        }
    }

    fn expect_identifier(&mut self, desc: &str) -> Result<String, ParseError> {
        let pos = self.current_pos();
        match self.peek() {
            Some(Token::Identifier(s)) => {
                let res = s.clone();
                self.cursor += 1;
                Ok(res)
            }
            _ => Err(ParseError::with_position(
                format!("Expected {}, found {:?}", desc, self.peek()),
                pos,
            )),
        }
    }

    // =========================================================================
    // Statement Dispatcher
    // =========================================================================

    fn parse_statement(&mut self) -> Result<SqlAction, ParseError> {
        let pos = self.current_pos();
        match self.peek() {
            Some(Token::Create) => self.parse_create(),
            Some(Token::Drop) => self.parse_drop(),
            Some(Token::Alter) => self.parse_alter(),
            Some(Token::Select) => {
                let query = self.parse_select_query()?;
                Ok(SqlAction::Select(Box::new(query)))
            }
            Some(Token::Insert) => self.parse_insert(),
            Some(Token::Update) => self.parse_update(),
            Some(Token::Delete) => self.parse_delete(),
            _ => Err(ParseError::with_position(
                format!("Expected SQL statement (CREATE, DROP, ALTER, SELECT, INSERT, UPDATE, DELETE), found {:?}", self.peek()),
                pos,
            )),
        }
    }

    // =========================================================================
    // DDL: CREATE & DROP
    // =========================================================================

    fn parse_create(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Create, "CREATE")?;

        if self.check(&Token::Unique) || self.check(&Token::Index) {
            let unique = self.consume(&Token::Unique);
            self.expect(&Token::Index, "INDEX")?;
            let if_not_exists = if self.consume(&Token::If) {
                self.expect(&Token::Not, "NOT")?;
                self.expect(&Token::Exists, "EXISTS")?;
                true
            } else {
                false
            };
            let name = self.expect_identifier("index name")?;
            self.expect(&Token::On, "ON")?;
            let table = self.expect_identifier("table name")?;
            self.expect(&Token::OpenParen, "(")?;
            let columns = self.parse_identifier_list()?;
            self.expect(&Token::CloseParen, ")")?;
            return Ok(SqlAction::CreateIndex {
                name,
                table,
                columns,
                unique,
                if_not_exists,
            });
        }
        if self.consume(&Token::Table) {
            let if_exists = if self.consume(&Token::If) {
                self.expect(&Token::Not, "NOT")?;
                self.expect(&Token::Exists, "EXISTS")?;
                true
            } else {
                false
            };

            let name = self.expect_identifier("table name")?;
            self.expect(&Token::OpenParen, "(")?;

            let mut entries = Vec::new();
            let mut key_constraints = Vec::new();
            let mut check = Vec::new();

            loop {
                if self.consume(&Token::Primary) {
                    self.expect(&Token::Key, "KEY")?;
                    self.expect(&Token::OpenParen, "(")?;
                    let cols = self.parse_identifier_list()?;
                    self.expect(&Token::CloseParen, ")")?;
                    key_constraints.push(KeyConstraint::PrimaryKey(cols));
                } else if self.consume(&Token::Unique) {
                    self.expect(&Token::OpenParen, "(")?;
                    let cols = self.parse_identifier_list()?;
                    self.expect(&Token::CloseParen, ")")?;
                    key_constraints.push(KeyConstraint::Unique(cols));
                } else if self.consume(&Token::Foreign) {
                    self.expect(&Token::Key, "KEY")?;
                    self.expect(&Token::OpenParen, "(")?;
                    let entry = self.parse_identifier_list()?;
                    self.expect(&Token::CloseParen, ")")?;
                    self.expect(&Token::References, "REFERENCES")?;
                    let referenced_table = self.expect_identifier("referenced table")?;
                    self.expect(&Token::OpenParen, "(")?;
                    let referenced_entries = self.parse_identifier_list()?;
                    self.expect(&Token::CloseParen, ")")?;
                    key_constraints.push(KeyConstraint::ForeignKey {
                        entry,
                        referenced_table,
                        referenced_entries,
                    });
                } else if self.consume(&Token::Check) {
                    self.expect(&Token::OpenParen, "(")?;
                    let pred = self.parse_predicate()?;
                    self.expect(&Token::CloseParen, ")")?;
                    check.push(pred);
                } else {
                    entries.push(self.parse_column_definition()?);
                }

                if !self.consume(&Token::Comma) {
                    break;
                }
            }

            self.expect(&Token::CloseParen, ")")?;
            Ok(SqlAction::CreateTable {
                name,
                entries,
                key_constraints,
                if_exists,
                check,
            })
        } else {
            let material = self.consume(&Token::Materialized);
            self.expect(&Token::View, "VIEW")?;
            let name = self.expect_identifier("view name")?;
            self.expect(&Token::As, "AS")?;
            let as_query = Box::new(self.parse_select_query()?);
            Ok(SqlAction::CreateView {
                name,
                material,
                as_query,
            })
        }
    }

    fn parse_column_definition(&mut self) -> Result<TupleSchemaEntry, ParseError> {
        let entry = self.expect_identifier("column name")?;
        let data_type = self.parse_data_type()?;
        let not_null = if self.consume(&Token::Not) {
            self.expect(&Token::Null, "NULL")?;
            true
        } else {
            false
        };
        let default = if self.consume(&Token::Default) {
            Some(self.parse_literal()?)
        } else {
            None
        };
        Ok(TupleSchemaEntry {
            entry,
            data_type,
            not_null,
            default,
        })
    }

    fn parse_rename_target(&mut self) -> Result<String, ParseError> {
        self.expect(&Token::To, "TO")?;
        self.expect_identifier("new name")
    }

    fn parse_alter(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Alter, "ALTER")?;
        if self.consume(&Token::Index) {
            let name = self.expect_identifier("index name")?;
            self.expect(&Token::Rename, "RENAME")?;
            let new_name = self.parse_rename_target()?;
            Ok(SqlAction::AlterIndex { name, new_name })
        } else if self.consume(&Token::View) {
            let name = self.expect_identifier("view name")?;
            let action = if self.consume(&Token::Rename) {
                AlterViewAction::RenameTo(self.parse_rename_target()?)
            } else {
                self.expect(&Token::As, "AS or RENAME")?;
                AlterViewAction::As(Box::new(self.parse_select_query()?))
            };
            Ok(SqlAction::AlterView { name, action })
        } else {
            self.expect(&Token::Table, "TABLE, VIEW or INDEX")?;
            let name = self.expect_identifier("table name")?;
            let action = if self.consume(&Token::Rename) {
                if self.consume(&Token::Column) {
                    let name = self.expect_identifier("column name")?;
                    let new_name = self.parse_rename_target()?;
                    AlterTableAction::RenameColumn { name, new_name }
                } else {
                    AlterTableAction::RenameTo(self.parse_rename_target()?)
                }
            } else if self.consume(&Token::Add) {
                self.consume(&Token::Column);
                AlterTableAction::AddColumn(self.parse_column_definition()?)
            } else if self.consume(&Token::Drop) {
                self.consume(&Token::Column);
                AlterTableAction::DropColumn(self.expect_identifier("column name")?)
            } else {
                self.expect(&Token::Alter, "ADD, DROP, RENAME or ALTER")?;
                self.consume(&Token::Column);
                let name = self.expect_identifier("column name")?;
                let action = if self.consume(&Token::Type) {
                    AlterColumnAction::SetDataType(self.parse_data_type()?)
                } else if self.consume(&Token::Set) {
                    if self.consume(&Token::Default) {
                        AlterColumnAction::SetDefault(self.parse_literal()?)
                    } else {
                        self.expect(&Token::Not, "DEFAULT or NOT NULL")?;
                        self.expect(&Token::Null, "NULL")?;
                        AlterColumnAction::SetNotNull
                    }
                } else {
                    self.expect(&Token::Drop, "TYPE, SET or DROP")?;
                    if self.consume(&Token::Default) {
                        AlterColumnAction::DropDefault
                    } else {
                        self.expect(&Token::Not, "DEFAULT or NOT NULL")?;
                        self.expect(&Token::Null, "NULL")?;
                        AlterColumnAction::DropNotNull
                    }
                };
                AlterTableAction::AlterColumn { name, action }
            };
            Ok(SqlAction::AlterTable { name, action })
        }
    }

    fn parse_data_type(&mut self) -> Result<TupleEntryDataType, ParseError> {
        let pos = self.current_pos();
        match self.peek() {
            Some(Token::Int) => {
                self.next();
                Ok(TupleEntryDataType::Int)
            }
            Some(Token::Float) => {
                self.next();
                Ok(TupleEntryDataType::Float)
            }
            Some(Token::Date) => {
                self.next();
                Ok(TupleEntryDataType::Date)
            }
            Some(Token::Time) => {
                self.next();
                Ok(TupleEntryDataType::Time)
            }
            Some(Token::Char) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let len = self.parse_u64()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(TupleEntryDataType::Char(len))
            }
            Some(Token::VarChar) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let len = self.parse_u64()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(TupleEntryDataType::VarChar(len))
            }
            _ => Err(ParseError::with_position(
                format!("Expected data type, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    fn parse_u64(&mut self) -> Result<u64, ParseError> {
        let pos = self.current_pos();
        match self.peek() {
            Some(Token::IntegerLiteral(n)) if *n >= 0 => {
                let val = *n as u64;
                self.next();
                Ok(val)
            }
            _ => Err(ParseError::with_position(
                format!("Expected positive integer, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    fn parse_drop(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Drop, "DROP")?;

        if self.consume(&Token::Table) {
            let if_exists = if self.consume(&Token::If) {
                self.expect(&Token::Exists, "EXISTS")?;
                true
            } else {
                false
            };
            let name = self.expect_identifier("table name")?;
            Ok(SqlAction::DropTable { name, if_exists })
        } else if self.consume(&Token::Index) {
            let if_exists = if self.consume(&Token::If) {
                self.expect(&Token::Exists, "EXISTS")?;
                true
            } else { false };
            let name = self.expect_identifier("index name")?;
            Ok(SqlAction::DropIndex { name, if_exists })
        } else if self.consume(&Token::View) {
            let if_exists = if self.consume(&Token::If) {
                self.expect(&Token::Exists, "EXISTS")?;
                true
            } else {
                false
            };
            let name = self.expect_identifier("view name")?;
            Ok(SqlAction::DropView { name, if_exists })
        } else {
            let pos = self.current_pos();
            Err(ParseError::with_position(
                format!("Expected TABLE, VIEW or INDEX after DROP, found {:?}", self.peek()),
                pos,
            ))
        }
    }

    // =========================================================================
    // DML: INSERT, UPDATE, DELETE
    // =========================================================================

    fn parse_insert(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Insert, "INSERT")?;
        self.expect(&Token::Into, "INTO")?;
        let table = self.expect_identifier("table name")?;

        let columns = if self.consume(&Token::OpenParen) {
            let cols = self.parse_identifier_list()?;
            self.expect(&Token::CloseParen, ")")?;
            cols
        } else {
            Vec::new()
        };

        if self.consume(&Token::Values) {
            let mut value = Vec::new();
            loop {
                self.expect(&Token::OpenParen, "(")?;
                let mut row = Vec::new();
                loop {
                    row.push(self.parse_expression()?);
                    if !self.consume(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::CloseParen, ")")?;
                value.push(row);
                if !self.consume(&Token::Comma) {
                    break;
                }
            }
            Ok(SqlAction::Insert {
                table,
                columns,
                value,
            })
        } else if self.check(&Token::Select) {
            let sql_query = Box::new(self.parse_select_query()?);
            Ok(SqlAction::InsertInto {
                table,
                columns,
                sql_query,
            })
        } else {
            let pos = self.current_pos();
            Err(ParseError::with_position(
                format!("Expected VALUES or SELECT after INSERT INTO, found {:?}", self.peek()),
                pos,
            ))
        }
    }

    fn parse_update(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Update, "UPDATE")?;
        let table = self.expect_identifier("table name")?;
        self.expect(&Token::Set, "SET")?;

        let mut value_map = HashMap::new();
        loop {
            let col = self.expect_identifier("column name")?;
            self.expect(&Token::Equals, "=")?;
            let expr = self.parse_expression()?;
            value_map.insert(col, Box::new(expr));
            if !self.consume(&Token::Comma) {
                break;
            }
        }

        let where_clause = if self.consume(&Token::Where) {
            Some(Box::new(self.parse_predicate()?))
        } else {
            None
        };

        Ok(SqlAction::Update {
            table,
            value_map,
            where_clause,
        })
    }

    fn parse_delete(&mut self) -> Result<SqlAction, ParseError> {
        self.expect(&Token::Delete, "DELETE")?;
        self.expect(&Token::From, "FROM")?;
        let table = self.expect_identifier("table name")?;

        let where_clause = if self.consume(&Token::Where) {
            Some(Box::new(self.parse_predicate()?))
        } else {
            None
        };

        Ok(SqlAction::Delete { table, where_clause })
    }

    // =========================================================================
    // DML: SELECT & Set Operations
    // =========================================================================

    pub fn parse_select_query(&mut self) -> Result<SqlQuery, ParseError> {
        let mut lhs = self.parse_basic_select()?;

        loop {
            if self.consume(&Token::Union) {
                if self.consume(&Token::All) {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::UnionAll(Box::new(lhs), Box::new(rhs));
                } else {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::Union(Box::new(lhs), Box::new(rhs));
                }
            } else if self.consume(&Token::Intersect) {
                if self.consume(&Token::All) {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::IntersectAll(Box::new(lhs), Box::new(rhs));
                } else {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::Intersect(Box::new(lhs), Box::new(rhs));
                }
            } else if self.consume(&Token::Except) {
                if self.consume(&Token::All) {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::ExceptAll(Box::new(lhs), Box::new(rhs));
                } else {
                    let rhs = self.parse_basic_select()?;
                    lhs = SqlQuery::Except(Box::new(lhs), Box::new(rhs));
                }
            } else {
                break;
            }
        }

        Ok(lhs)
    }

    fn parse_basic_select(&mut self) -> Result<SqlQuery, ParseError> {
        self.expect(&Token::Select, "SELECT")?;
        let distinct = self.consume(&Token::Distinct);

        let mut select = Vec::new();
        loop {
            let expr = self.parse_expression()?;
            let alias = if self.consume(&Token::As) {
                Some(self.expect_identifier("column alias")?)
            } else if self.is_non_keyword_identifier() {
                Some(self.expect_identifier("column alias")?)
            } else {
                None
            };
            select.push(SelectItem { expr, alias });
            if !self.consume(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::From, "FROM")?;
        let from = self.parse_table_ref()?;

        let where_clause = if self.consume(&Token::Where) {
            Some(self.parse_predicate()?)
        } else {
            None
        };

        let group_by = if self.consume(&Token::Group) {
            self.expect(&Token::By, "BY")?;
            let group_values = self.parse_identifier_list()?;
            let having_clause = if self.consume(&Token::Having) {
                Some(self.parse_predicate()?)
            } else {
                None
            };
            Some(GroupBy {
                group_values,
                having_clause,
            })
        } else {
            None
        };

        let order = if self.consume(&Token::Order) {
            self.expect(&Token::By, "BY")?;
            let mut values = Vec::new();
            loop {
                let alias = self.expect_identifier("order column")?;
                let order = if self.consume(&Token::Desc) {
                    Order::Desc
                } else if self.consume(&Token::Asc) {
                    Order::Asc
                } else {
                    Order::Asc
                };
                values.push(OrderClause { alias, order });
                if !self.consume(&Token::Comma) {
                    break;
                }
            }
            Some(OrderBy { values })
        } else {
            None
        };

        Ok(SqlQuery::Basic(BasicSqlQuery {
            select,
            distinct,
            from,
            where_clause,
            group_by,
            order,
        }))
    }

    fn is_non_keyword_identifier(&self) -> bool {
        matches!(self.peek(), Some(Token::Identifier(_)))
    }

    // =========================================================================
    // Table References & Joins
    // =========================================================================

    fn parse_table_ref(&mut self) -> Result<TableRef, ParseError> {
        let mut current = self.parse_single_table_ref()?;

        loop {
            if self.consume(&Token::Comma) {
                // Comma implicit cross join
                let other = self.parse_single_table_ref()?;
                current = TableRef::Join {
                    relation: Box::new(current),
                    join_type: JoinType::CrossJoin(Box::new(other)),
                };
            } else if self.consume(&Token::Cross) {
                self.expect(&Token::Join, "JOIN")?;
                let other = self.parse_single_table_ref()?;
                current = TableRef::Join {
                    relation: Box::new(current),
                    join_type: JoinType::CrossJoin(Box::new(other)),
                };
            } else if self.check(&Token::Join)
                || self.check(&Token::Inner)
                || self.check(&Token::Left)
                || self.check(&Token::Right)
                || self.check(&Token::Full)
            {
                let join_kind = self.parse_join_type_prefix()?;
                let other = self.parse_single_table_ref()?;
                self.expect(&Token::On, "ON")?;
                let condition = self.parse_predicate()?;

                let join_type = match join_kind {
                    JoinKind::Inner => JoinType::InnerJoin {
                        other: Box::new(other),
                        condition,
                    },
                    JoinKind::Left => JoinType::LeftJoin {
                        other: Box::new(other),
                        condition,
                    },
                    JoinKind::Right => JoinType::RightJoin {
                        other: Box::new(other),
                        condition,
                    },
                    JoinKind::Full => JoinType::FullJoin {
                        other: Box::new(other),
                        condition,
                    },
                };

                current = TableRef::Join {
                    relation: Box::new(current),
                    join_type,
                };
            } else {
                break;
            }
        }

        Ok(current)
    }

    fn parse_join_type_prefix(&mut self) -> Result<JoinKind, ParseError> {
        if self.consume(&Token::Inner) {
            self.expect(&Token::Join, "JOIN")?;
            Ok(JoinKind::Inner)
        } else if self.consume(&Token::Left) {
            self.consume(&Token::Outer);
            self.expect(&Token::Join, "JOIN")?;
            Ok(JoinKind::Left)
        } else if self.consume(&Token::Right) {
            self.consume(&Token::Outer);
            self.expect(&Token::Join, "JOIN")?;
            Ok(JoinKind::Right)
        } else if self.consume(&Token::Full) {
            self.consume(&Token::Outer);
            self.expect(&Token::Join, "JOIN")?;
            Ok(JoinKind::Full)
        } else if self.consume(&Token::Join) {
            Ok(JoinKind::Inner)
        } else {
            let pos = self.current_pos();
            Err(ParseError::with_position("Expected join keyword", pos))
        }
    }

    fn parse_single_table_ref(&mut self) -> Result<TableRef, ParseError> {
        if self.consume(&Token::OpenParen) {
            let query = Box::new(self.parse_select_query()?);
            self.expect(&Token::CloseParen, ")")?;
            self.consume(&Token::As);
            let alias = self.expect_identifier("subquery alias")?;
            Ok(TableRef::SubQuery { query, alias })
        } else {
            let name = self.expect_identifier("table name")?;
            let alias = if self.consume(&Token::As) {
                Some(self.expect_identifier("table alias")?)
            } else if self.is_table_alias() {
                Some(self.expect_identifier("table alias")?)
            } else {
                None
            };
            Ok(TableRef::Table { name, alias })
        }
    }

    fn is_table_alias(&self) -> bool {
        match self.peek() {
            Some(Token::Identifier(_)) => {
                // An identifier following table name is an alias unless it's a clause keyword
                true
            }
            _ => false,
        }
    }

    // =========================================================================
    // Predicates
    // =========================================================================

    fn parse_predicate(&mut self) -> Result<Predicate, ParseError> {
        self.parse_or_predicate()
    }

    fn parse_or_predicate(&mut self) -> Result<Predicate, ParseError> {
        let mut lhs = self.parse_and_predicate()?;
        while self.consume(&Token::Or) {
            let rhs = self.parse_and_predicate()?;
            lhs = Predicate::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and_predicate(&mut self) -> Result<Predicate, ParseError> {
        let mut lhs = self.parse_not_predicate()?;
        while self.consume(&Token::And) {
            let rhs = self.parse_not_predicate()?;
            lhs = Predicate::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_not_predicate(&mut self) -> Result<Predicate, ParseError> {
        if self.consume(&Token::Not) {
            let inner = self.parse_atomic_predicate()?;
            Ok(Predicate::Not(Box::new(inner)))
        } else {
            self.parse_atomic_predicate()
        }
    }

    fn is_parenthesized_predicate(&self) -> bool {
        if !self.check(&Token::OpenParen) {
            return false;
        }
        let mut depth = 0;
        let mut i = self.cursor;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::OpenParen => depth += 1,
                Token::CloseParen => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Token::Select if depth == 1 => return false,
                Token::And | Token::Or if depth == 1 => return true,
                Token::Equals
                | Token::NotEquals
                | Token::LessThan
                | Token::LessThanEquals
                | Token::GreaterThan
                | Token::GreaterThanEquals
                | Token::Is
                | Token::Between
                | Token::Like
                | Token::In if depth == 1 => return true,
                _ => {}
            }
            i += 1;
        }
        false
    }

    fn parse_atomic_predicate(&mut self) -> Result<Predicate, ParseError> {
        if self.is_parenthesized_predicate() {
            self.expect(&Token::OpenParen, "(")?;
            let pred = self.parse_predicate()?;
            self.expect(&Token::CloseParen, ")")?;
            return Ok(pred);
        }

        if self.consume(&Token::Exists) {
            self.expect(&Token::OpenParen, "(")?;
            let query = self.parse_select_query()?;
            self.expect(&Token::CloseParen, ")")?;
            return Ok(Predicate::Exists(Box::new(SubQuery {
                sub_query: Box::new(query),
                imported_variables: Vec::new(),
            })));
        }

        let lhs = self.parse_expression()?;

        if self.consume(&Token::Is) {
            if self.consume(&Token::Not) {
                self.expect(&Token::Null, "NULL")?;
                Ok(Predicate::IsNotNull(Box::new(lhs)))
            } else {
                self.expect(&Token::Null, "NULL")?;
                Ok(Predicate::IsNull(Box::new(lhs)))
            }
        } else if self.consume(&Token::Like) {
            let pattern = self.expect_string_literal()?;
            Ok(Predicate::Like(Box::new(lhs), pattern))
        } else if self.consume(&Token::Between) {
            let low = self.parse_expression()?;
            self.expect(&Token::And, "AND")?;
            let high = self.parse_expression()?;
            Ok(Predicate::Between(
                Box::new(lhs),
                Box::new(low),
                Box::new(high),
            ))
        } else if self.consume(&Token::In) {
            self.expect(&Token::OpenParen, "(")?;
            if self.check(&Token::Select) {
                let query = self.parse_select_query()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Predicate::In(
                    Box::new(lhs),
                    InValue::Query(SubQuery {
                        sub_query: Box::new(query),
                        imported_variables: Vec::new(),
                    }),
                ))
            } else {
                let mut list = Vec::new();
                loop {
                    list.push(self.parse_literal()?);
                    if !self.consume(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::CloseParen, ")")?;
                Ok(Predicate::In(Box::new(lhs), InValue::List(list)))
            }
        } else if self.is_comparison_op() {
            let comp = self.parse_comparison_op()?;
            if self.consume(&Token::All) {
                self.expect(&Token::OpenParen, "(")?;
                let query = self.parse_select_query()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Predicate::All(
                    comp,
                    Box::new(lhs),
                    Box::new(SubQuery {
                        sub_query: Box::new(query),
                        imported_variables: Vec::new(),
                    }),
                ))
            } else if self.consume(&Token::Any) {
                self.expect(&Token::OpenParen, "(")?;
                let query = self.parse_select_query()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Predicate::Any(
                    comp,
                    Box::new(lhs),
                    Box::new(SubQuery {
                        sub_query: Box::new(query),
                        imported_variables: Vec::new(),
                    }),
                ))
            } else {
                let rhs = self.parse_expression()?;
                Ok(Predicate::Compare(comp, Box::new(lhs), Box::new(rhs)))
            }
        } else {
            // Check for standalone boolean expressions
            match lhs {
                Expression::Constant(Literal::Boolean(true)) => Ok(Predicate::True),
                Expression::Constant(Literal::Boolean(false)) => Ok(Predicate::False),
                _ => {
                    let pos = self.current_pos();
                    Err(ParseError::with_position(
                        format!("Expected predicate condition, found {:?}", self.peek()),
                        pos,
                    ))
                }
            }
        }
    }

    fn is_comparison_op(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Equals)
                | Some(Token::NotEquals)
                | Some(Token::LessThan)
                | Some(Token::LessThanEquals)
                | Some(Token::GreaterThan)
                | Some(Token::GreaterThanEquals)
        )
    }

    fn parse_comparison_op(&mut self) -> Result<Comparison, ParseError> {
        let pos = self.current_pos();
        match self.peek() {
            Some(Token::Equals) => {
                self.next();
                Ok(Comparison::Equals)
            }
            Some(Token::NotEquals) => {
                self.next();
                Ok(Comparison::NotEquals)
            }
            Some(Token::LessThan) => {
                self.next();
                Ok(Comparison::LessThan)
            }
            Some(Token::LessThanEquals) => {
                self.next();
                Ok(Comparison::LessThanEquals)
            }
            Some(Token::GreaterThan) => {
                self.next();
                Ok(Comparison::GreaterThan)
            }
            Some(Token::GreaterThanEquals) => {
                self.next();
                Ok(Comparison::GreaterThanEquals)
            }
            _ => Err(ParseError::with_position(
                format!("Expected comparison operator, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    // =========================================================================
    // Expressions (Pratt Precedence Climbing)
    // =========================================================================

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_expr_with_min_bp(0)
    }

    fn parse_expr_with_min_bp(&mut self, min_bp: u8) -> Result<Expression, ParseError> {
        let mut lhs = self.parse_prefix_expr()?;

        loop {
            let (l_bp, r_bp, op) = match self.peek() {
                Some(Token::Plus) => (1, 2, BinaryOp::Add),
                Some(Token::Minus) => (1, 2, BinaryOp::Subtract),
                Some(Token::Asterisk) => (3, 4, BinaryOp::Multiply),
                Some(Token::Slash) => (3, 4, BinaryOp::Divide),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.next(); // consume operator
            let rhs = self.parse_expr_with_min_bp(r_bp)?;

            lhs = match op {
                BinaryOp::Add => Expression::Add(Box::new(lhs), Box::new(rhs)),
                BinaryOp::Subtract => Expression::Subtract(Box::new(lhs), Box::new(rhs)),
                BinaryOp::Multiply => Expression::Multiply(Box::new(lhs), Box::new(rhs)),
                BinaryOp::Divide => Expression::Divide(Box::new(lhs), Box::new(rhs)),
            };
        }

        Ok(lhs)
    }

    fn parse_prefix_expr(&mut self) -> Result<Expression, ParseError> {
        if self.consume(&Token::Minus) {
            // Unary negation has high precedence (5)
            let inner = self.parse_expr_with_min_bp(5)?;
            Ok(Expression::Negate(Box::new(inner)))
        } else {
            self.parse_primary_expr()
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expression, ParseError> {
        let pos = self.current_pos();
        match self.peek().cloned() {
            Some(Token::Parameter) => {
                self.next();
                self.parameter_count += 1;
                Ok(Expression::Parameter(self.parameter_count))
            }
            Some(Token::IntegerLiteral(n)) => {
                self.next();
                Ok(Expression::Constant(Literal::Integer(n)))
            }
            Some(Token::FloatLiteral(f)) => {
                self.next();
                Ok(Expression::Constant(Literal::Float(f)))
            }
            Some(Token::StringLiteral(s)) => {
                self.next();
                Ok(Expression::Constant(Literal::String(s)))
            }
            Some(Token::True) => {
                self.next();
                Ok(Expression::Constant(Literal::Boolean(true)))
            }
            Some(Token::False) => {
                self.next();
                Ok(Expression::Constant(Literal::Boolean(false)))
            }
            Some(Token::Null) => {
                self.next();
                Ok(Expression::Constant(Literal::Null))
            }
            Some(Token::Asterisk) => {
                self.next();
                Ok(Expression::Variable {
                    name: "*".to_string(),
                    table: None,
                })
            }
            Some(Token::Count) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let agg = if self.consume(&Token::Asterisk) {
                    AggregateValues::CountStar
                } else {
                    let distinct = self.consume(&Token::Distinct);
                    let expr = self.parse_expression()?;
                    AggregateValues::Count {
                        expression: Box::new(expr),
                        distinct,
                    }
                };
                self.expect(&Token::CloseParen, ")")?;
                Ok(Expression::Aggregate(agg))
            }
            Some(Token::Sum) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let distinct = self.consume(&Token::Distinct);
                let expr = self.parse_expression()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Expression::Aggregate(AggregateValues::Sum {
                    expression: Box::new(expr),
                    distinct,
                }))
            }
            Some(Token::Avg) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let distinct = self.consume(&Token::Distinct);
                let expr = self.parse_expression()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Expression::Aggregate(AggregateValues::Avg {
                    expression: Box::new(expr),
                    distinct,
                }))
            }
            Some(Token::Min) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let expr = self.parse_expression()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Expression::Aggregate(AggregateValues::Min(Box::new(expr))))
            }
            Some(Token::Max) => {
                self.next();
                self.expect(&Token::OpenParen, "(")?;
                let expr = self.parse_expression()?;
                self.expect(&Token::CloseParen, ")")?;
                Ok(Expression::Aggregate(AggregateValues::Max(Box::new(expr))))
            }
            Some(Token::Identifier(name)) => {
                self.next();
                if self.consume(&Token::OpenParen) {
                    let mut args = Vec::new();
                    if !self.check(&Token::CloseParen) {
                        loop {
                            args.push(Box::new(self.parse_expression()?));
                            if !self.consume(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::CloseParen, ")")?;
                    Ok(Expression::Function { name, args })
                } else if self.consume(&Token::Dot) {
                    if self.consume(&Token::Asterisk) {
                        Ok(Expression::Variable {
                            name: "*".to_string(),
                            table: Some(name),
                        })
                    } else {
                        let col = self.expect_identifier("column name")?;
                        Ok(Expression::Variable {
                            name: col,
                            table: Some(name),
                        })
                    }
                } else {
                    Ok(Expression::Variable {
                        name,
                        table: None,
                    })
                }
            }
            Some(Token::OpenParen) => {
                self.next();
                if self.check(&Token::Select) {
                    let query = self.parse_select_query()?;
                    self.expect(&Token::CloseParen, ")")?;
                    Ok(Expression::SubQuery(SubQuery {
                        sub_query: Box::new(query),
                        imported_variables: Vec::new(),
                    }))
                } else {
                    let expr = self.parse_expression()?;
                    self.expect(&Token::CloseParen, ")")?;
                    Ok(expr)
                }
            }
            _ => Err(ParseError::with_position(
                format!("Expected expression, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    // =========================================================================
    // Helpers: Literals & Lists
    // =========================================================================

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        let pos = self.current_pos();
        match self.peek().cloned() {
            Some(Token::IntegerLiteral(n)) => {
                self.next();
                Ok(Literal::Integer(n))
            }
            Some(Token::FloatLiteral(f)) => {
                self.next();
                Ok(Literal::Float(f))
            }
            Some(Token::StringLiteral(s)) => {
                self.next();
                Ok(Literal::String(s))
            }
            Some(Token::True) => {
                self.next();
                Ok(Literal::Boolean(true))
            }
            Some(Token::False) => {
                self.next();
                Ok(Literal::Boolean(false))
            }
            Some(Token::Null) => {
                self.next();
                Ok(Literal::Null)
            }
            _ => Err(ParseError::with_position(
                format!("Expected literal, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    fn expect_string_literal(&mut self) -> Result<String, ParseError> {
        let pos = self.current_pos();
        match self.peek().cloned() {
            Some(Token::StringLiteral(s)) => {
                self.next();
                Ok(s)
            }
            _ => Err(ParseError::with_position(
                format!("Expected string literal, found {:?}", self.peek()),
                pos,
            )),
        }
    }

    fn parse_identifier_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut list = Vec::new();
        loop {
            list.push(self.expect_identifier("identifier")?);
            if !self.consume(&Token::Comma) {
                break;
            }
        }
        Ok(list)
    }
}

enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
}
