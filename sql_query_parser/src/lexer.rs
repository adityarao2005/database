use crate::error::ParseError;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords - DDL & Statements
    Create,
    Index,
    Alter,
    Add,
    Column,
    Rename,
    To,
    Type,
    Table,
    View,
    Materialized,
    Drop,
    If,
    Not,
    Exists,
    Primary,
    Key,
    Unique,
    Foreign,
    References,
    Check,
    Default,

    // Keywords - DML
    Select,
    Distinct,
    From,
    Where,
    Group,
    By,
    Having,
    Order,
    Asc,
    Desc,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,

    // Keywords - Joins & Set Operations
    Join,
    Inner,
    Left,
    Right,
    Full,
    Outer,
    Cross,
    On,
    As,
    Union,
    Intersect,
    Except,
    All,
    Any,

    // Keywords - Predicates & Logic
    And,
    Or,
    Between,
    Like,
    Is,
    In,

    // Keywords - Data Types
    Int,
    Float,
    Date,
    Time,
    Char,
    VarChar,

    // Keywords - Aggregates
    Count,
    Sum,
    Avg,
    Min,
    Max,

    // Literals
    True,
    False,
    Null,
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),

    // Identifiers
    Identifier(String),

    // Punctuation & Delimiters
    OpenParen,
    CloseParen,
    Comma,
    Semicolon,
    Dot,

    // Arithmetic & Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}

impl SpannedToken {
    pub fn new(token: Token, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Skip whitespace
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
            i += 1;
            continue;
        }

        // Single-line comment (-- ...)
        if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Multi-line comment (/* ... */)
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            let mut closed = false;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return Err(ParseError::with_position("Unterminated block comment", start));
            }
            continue;
        }

        let start = i;

        // Two-character operators
        if i + 1 < len {
            match (b, bytes[i + 1]) {
                (b'!', b'=') | (b'<', b'>') => {
                    tokens.push(SpannedToken::new(Token::NotEquals, start, start + 2));
                    i += 2;
                    continue;
                }
                (b'<', b'=') => {
                    tokens.push(SpannedToken::new(Token::LessThanEquals, start, start + 2));
                    i += 2;
                    continue;
                }
                (b'>', b'=') => {
                    tokens.push(SpannedToken::new(Token::GreaterThanEquals, start, start + 2));
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        // Single-character punctuation & operators
        match b {
            b'(' => {
                tokens.push(SpannedToken::new(Token::OpenParen, start, start + 1));
                i += 1;
                continue;
            }
            b')' => {
                tokens.push(SpannedToken::new(Token::CloseParen, start, start + 1));
                i += 1;
                continue;
            }
            b',' => {
                tokens.push(SpannedToken::new(Token::Comma, start, start + 1));
                i += 1;
                continue;
            }
            b';' => {
                tokens.push(SpannedToken::new(Token::Semicolon, start, start + 1));
                i += 1;
                continue;
            }
            b'.' => {
                tokens.push(SpannedToken::new(Token::Dot, start, start + 1));
                i += 1;
                continue;
            }
            b'+' => {
                tokens.push(SpannedToken::new(Token::Plus, start, start + 1));
                i += 1;
                continue;
            }
            b'-' => {
                tokens.push(SpannedToken::new(Token::Minus, start, start + 1));
                i += 1;
                continue;
            }
            b'*' => {
                tokens.push(SpannedToken::new(Token::Asterisk, start, start + 1));
                i += 1;
                continue;
            }
            b'/' => {
                tokens.push(SpannedToken::new(Token::Slash, start, start + 1));
                i += 1;
                continue;
            }
            b'=' => {
                tokens.push(SpannedToken::new(Token::Equals, start, start + 1));
                i += 1;
                continue;
            }
            b'<' => {
                tokens.push(SpannedToken::new(Token::LessThan, start, start + 1));
                i += 1;
                continue;
            }
            b'>' => {
                tokens.push(SpannedToken::new(Token::GreaterThan, start, start + 1));
                i += 1;
                continue;
            }
            _ => {}
        }

        // String literals: '...' (with '' escape)
        if b == b'\'' {
            let mut s = String::new();
            i += 1;
            let mut terminated = false;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        s.push('\'');
                        i += 2;
                    } else {
                        i += 1;
                        terminated = true;
                        break;
                    }
                } else {
                    let ch = input[i..].chars().next().unwrap();
                    s.push(ch);
                    i += ch.len_utf8();
                }
            }
            if !terminated {
                return Err(ParseError::with_position("Unterminated string literal", start));
            }
            tokens.push(SpannedToken::new(Token::StringLiteral(s), start, i));
            continue;
        }

        // Numeric literals: digits optionally followed by . digits
        if b.is_ascii_digit() {
            let num_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }

            // Check if this is a float (e.g. 1.1 or 0.0)
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                i += 1; // skip '.'
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let num_str = &input[num_start..i];
                let val: f64 = num_str.parse().map_err(|_| {
                    ParseError::with_position(format!("Invalid float literal: {}", num_str), num_start)
                })?;
                tokens.push(SpannedToken::new(Token::FloatLiteral(val), num_start, i));
            } else {
                let num_str = &input[num_start..i];
                let val: i64 = num_str.parse().map_err(|_| {
                    ParseError::with_position(format!("Invalid integer literal: {}", num_str), num_start)
                })?;
                tokens.push(SpannedToken::new(Token::IntegerLiteral(val), num_start, i));
            }
            continue;
        }

        // Identifiers and Keywords
        if b.is_ascii_alphabetic() || b == b'_' {
            let id_start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &input[id_start..i];
            let upper = word.to_ascii_uppercase();

            let token = match upper.as_str() {
                "CREATE" => Token::Create,
                "INDEX" => Token::Index,
                "ALTER" => Token::Alter,
                "ADD" => Token::Add,
                "COLUMN" => Token::Column,
                "RENAME" => Token::Rename,
                "TO" => Token::To,
                "TYPE" => Token::Type,

                "TABLE" => Token::Table,
                "VIEW" => Token::View,
                "MATERIALIZED" => Token::Materialized,
                "DROP" => Token::Drop,
                "IF" => Token::If,
                "NOT" => Token::Not,
                "EXISTS" => Token::Exists,
                "PRIMARY" => Token::Primary,
                "KEY" => Token::Key,
                "UNIQUE" => Token::Unique,
                "FOREIGN" => Token::Foreign,
                "REFERENCES" => Token::References,
                "CHECK" => Token::Check,
                "DEFAULT" => Token::Default,

                "SELECT" => Token::Select,
                "DISTINCT" => Token::Distinct,
                "FROM" => Token::From,
                "WHERE" => Token::Where,
                "GROUP" => Token::Group,
                "BY" => Token::By,
                "HAVING" => Token::Having,
                "ORDER" => Token::Order,
                "ASC" => Token::Asc,
                "DESC" => Token::Desc,
                "INSERT" => Token::Insert,
                "INTO" => Token::Into,
                "VALUES" => Token::Values,
                "UPDATE" => Token::Update,
                "SET" => Token::Set,
                "DELETE" => Token::Delete,

                "JOIN" => Token::Join,
                "INNER" => Token::Inner,
                "LEFT" => Token::Left,
                "RIGHT" => Token::Right,
                "FULL" => Token::Full,
                "OUTER" => Token::Outer,
                "CROSS" => Token::Cross,
                "ON" => Token::On,
                "AS" => Token::As,
                "UNION" => Token::Union,
                "INTERSECT" => Token::Intersect,
                "EXCEPT" => Token::Except,
                "ALL" => Token::All,
                "ANY" => Token::Any,

                "AND" => Token::And,
                "OR" => Token::Or,
                "BETWEEN" => Token::Between,
                "LIKE" => Token::Like,
                "IS" => Token::Is,
                "IN" => Token::In,

                "INT" => Token::Int,
                "FLOAT" => Token::Float,
                "DATE" => Token::Date,
                "TIME" => Token::Time,
                "CHAR" => Token::Char,
                "VARCHAR" => Token::VarChar,

                "COUNT" => Token::Count,
                "SUM" => Token::Sum,
                "AVG" => Token::Avg,
                "MIN" => Token::Min,
                "MAX" => Token::Max,

                "TRUE" => Token::True,
                "FALSE" => Token::False,
                "NULL" => Token::Null,

                _ => Token::Identifier(word.to_string()),
            };

            tokens.push(SpannedToken::new(token, id_start, i));
            continue;
        }

        // Quoted identifiers e.g. "users" or `users`
        if b == b'"' || b == b'`' {
            let quote = b;
            i += 1;
            let id_start = i;
            while i < len && bytes[i] != quote {
                i += 1;
            }
            if i >= len {
                return Err(ParseError::with_position("Unterminated quoted identifier", start));
            }
            let word = &input[id_start..i];
            i += 1; // skip closing quote
            tokens.push(SpannedToken::new(Token::Identifier(word.to_string()), start, i));
            continue;
        }

        return Err(ParseError::with_position(
            format!("Unexpected character: '{}'", b as char),
            start,
        ));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_keywords_case_insensitive() {
        let sql = "select DISTINCT FROM where GROUP BY having ORDER BY asc desc";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Select,
                Token::Distinct,
                Token::From,
                Token::Where,
                Token::Group,
                Token::By,
                Token::Having,
                Token::Order,
                Token::By,
                Token::Asc,
                Token::Desc,
            ]
        );
    }

    #[test]
    fn test_tokenize_ddl_and_types() {
        let sql = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(255), rating FLOAT DEFAULT 0.0);";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Create,
                Token::Table,
                Token::Identifier("users".to_string()),
                Token::OpenParen,
                Token::Identifier("id".to_string()),
                Token::Int,
                Token::Not,
                Token::Null,
                Token::Comma,
                Token::Identifier("name".to_string()),
                Token::VarChar,
                Token::OpenParen,
                Token::IntegerLiteral(255),
                Token::CloseParen,
                Token::Comma,
                Token::Identifier("rating".to_string()),
                Token::Float,
                Token::Default,
                Token::FloatLiteral(0.0),
                Token::CloseParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn test_tokenize_operators_and_literals() {
        let sql = "price * 1.1 + 5 - -discount / 2 != 0 AND status = 'active'";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("price".to_string()),
                Token::Asterisk,
                Token::FloatLiteral(1.1),
                Token::Plus,
                Token::IntegerLiteral(5),
                Token::Minus,
                Token::Minus,
                Token::Identifier("discount".to_string()),
                Token::Slash,
                Token::IntegerLiteral(2),
                Token::NotEquals,
                Token::IntegerLiteral(0),
                Token::And,
                Token::Identifier("status".to_string()),
                Token::Equals,
                Token::StringLiteral("active".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenize_string_with_escaped_quote() {
        let sql = "'It''s a SQL test'";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(tokens, vec![Token::StringLiteral("It's a SQL test".to_string())]);
    }

    #[test]
    fn test_tokenize_joins_and_comparisons() {
        let sql = "a <= 10 AND b >= 20 AND c <> 30 AND d < 5 AND e > 1";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::LessThanEquals,
                Token::IntegerLiteral(10),
                Token::And,
                Token::Identifier("b".to_string()),
                Token::GreaterThanEquals,
                Token::IntegerLiteral(20),
                Token::And,
                Token::Identifier("c".to_string()),
                Token::NotEquals,
                Token::IntegerLiteral(30),
                Token::And,
                Token::Identifier("d".to_string()),
                Token::LessThan,
                Token::IntegerLiteral(5),
                Token::And,
                Token::Identifier("e".to_string()),
                Token::GreaterThan,
                Token::IntegerLiteral(1),
            ]
        );
    }

    #[test]
    fn test_tokenize_comments() {
        let sql = "SELECT -- single line\nid /* multi\nline */ FROM users;";
        let tokens: Vec<Token> = tokenize(sql).unwrap().into_iter().map(|t| t.token).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Select,
                Token::Identifier("id".to_string()),
                Token::From,
                Token::Identifier("users".to_string()),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn test_tokenize_spans() {
        let sql = "SELECT id";
        let tokens = tokenize(sql).unwrap();
        assert_eq!(tokens[0], SpannedToken::new(Token::Select, 0, 6));
        assert_eq!(tokens[1], SpannedToken::new(Token::Identifier("id".to_string()), 7, 9));
    }

    #[test]
    fn test_tokenize_error_unterminated_string() {
        let sql = "SELECT 'unterminated";
        let err = tokenize(sql).unwrap_err();
        assert_eq!(err.position, Some(7));
    }
}
