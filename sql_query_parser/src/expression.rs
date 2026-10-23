use crate::dml::select::SqlQuery;


pub enum Expression {
    Constant(String),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Function {
        name: String,
        args: Vec<Box<Expression>>
    },
    Variable {
        name: String,
        table: Option<String>
    },
    SubQuery {
        sub_query: Box<SqlQuery>,
        imported_variables: Vec<String>
    }
}

pub enum Predicate {
    Equals(Expression, Expression),
    GreaterThan(Expression, Expression),
    LessThan(Expression, Expression),
    GreatherThanOrEquals(Expression, Expression),
    LessThanOrEquals(Expression, Expression),
    NotEquals(Expression, Expression),
    Between(Expression, Expression, Expression),
    AND(Box<Predicate>, Box<Predicate>),
    OR(Box<Predicate>, Box<Predicate>),
    NOT(Box<Predicate>),
}