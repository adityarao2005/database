use crate::dml::select::SqlQuery;

pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,

}

pub enum AggregateValues {
    Distinct(String),
    Count {
        var: String,
        distinct: bool
    },
    Sum {
        var: String,
        distinct: bool
    },
    Avg {
        var: String,
        distinct: bool
    }
}

pub enum Expression {
    Constant(Literal),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Negate(Box<Expression>),
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
    },
    Aggregate(AggregateValues)
}

pub enum InValue {
    List(Vec<Literal>),
    Query(Box<SqlQuery>)
}

pub enum Comparison {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
}

pub enum Predicate {
    Compare(Comparison, Expression, Expression),
    
    Between(Expression, Expression, Expression),
    
    IsNull(Expression),
    IsNotNull(Expression),
    
    Like(Expression, String),

    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),

    Exists(Expression, Box<SqlQuery>),
    In(Expression, InValue),
    NotIn(Expression, Box<SqlQuery>),

    All(Comparison, Expression, Expression),
    Any(Comparison, Expression, Expression),

    True,
    False
}