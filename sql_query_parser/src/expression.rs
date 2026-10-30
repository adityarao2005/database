use crate::dml::select::{SubQuery};

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AggregateValues {
    Count {
        expression: Box<Expression>,
        distinct: bool,
    },
    CountStar,
    Sum {
        expression: Box<Expression>,
        distinct: bool,
    },
    Avg {
        expression: Box<Expression>,
        distinct: bool,
    },
    Min(Box<Expression>),
    Max(Box<Expression>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    /// One-based position in statement source order, including nested queries.
    Parameter(usize),
    Constant(Literal),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Negate(Box<Expression>),
    Function {
        name: String,
        args: Vec<Box<Expression>>,
    },
    Variable {
        name: String,
        table: Option<String>,
    },
    SubQuery(SubQuery),
    Aggregate(AggregateValues),
}

#[derive(Debug, PartialEq, Clone)]
pub enum InValue {
    List(Vec<Literal>),
    Query(SubQuery),
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum Comparison {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Predicate {
    Compare(Comparison, Box<Expression>, Box<Expression>),

    Between(Box<Expression>, Box<Expression>, Box<Expression>),

    IsNull(Box<Expression>),
    IsNotNull(Box<Expression>),

    Like(Box<Expression>, String),

    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),

    Exists(Box<SubQuery>),
    In(Box<Expression>, InValue),

    All(Comparison, Box<Expression>, Box<SubQuery>),
    Any(Comparison, Box<Expression>, Box<SubQuery>),

    True,
    False,
}
