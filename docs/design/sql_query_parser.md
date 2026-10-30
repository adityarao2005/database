# SQL Query Lexer and Parser

## Grammar for Lexer

```ebnf
/* =========================================================================
 * 1. WHITESPACE & TOKENS
 * ========================================================================= */
WHITESPACE := "" | (" " + WHITESPACE) | ("\t" + WHITESPACE) | ("\n" + WHITESPACE) | ("\r" + WHITESPACE)
WS := WHITESPACE
WS_REQ := (" " | "\t" | "\n" | "\r") + WHITESPACE

IDENTIFIER  := {identifier}
TABLE_NAME  := {tablename}
COLUMN_NAME := {columnname}
VIEW_NAME   := {viewname}

/* =========================================================================
 * 2. TOP-LEVEL STATEMENTS
 * ========================================================================= */
STATEMENT := WS + OPERATION + WS + (";" + WS)?

OPERATION := CREATE
           | DROP
           | SELECT_QUERY
           | INSERT
           | UPDATE
           | DELETE

/* =========================================================================
 * 3. DDL (DATA DEFINITION LANGUAGE)
 * ========================================================================= */
CREATE := "CREATE" + WS_REQ + (CREATE_TABLE | CREATE_VIEW)

CREATE_TABLE := "TABLE" + WS_REQ + ("IF" + WS_REQ + "NOT" + WS_REQ + "EXISTS" + WS_REQ)? + TABLE_NAME + WS + "(" + WS + TABLE_ELEMENTS + WS + ")"

TABLE_ELEMENTS := TABLE_ELEMENT + (WS + "," + WS + TABLE_ELEMENT)*
TABLE_ELEMENT  := COLUMN_DEF | TABLE_CONSTRAINT

COLUMN_DEF := COLUMN_NAME + WS_REQ + DATA_TYPE + (WS_REQ + "NOT" + WS_REQ + "NULL")? + (WS_REQ + "DEFAULT" + WS_REQ + LITERAL)?

DATA_TYPE := "INT"
           | "FLOAT"
           | "DATE"
           | "TIME"
           | "CHAR" + WS + "(" + WS + {integer} + WS + ")"
           | "VARCHAR" + WS + "(" + WS + {integer} + WS + ")"

TABLE_CONSTRAINT := PRIMARY_KEY_CONSTRAINT
                  | UNIQUE_CONSTRAINT
                  | FOREIGN_KEY_CONSTRAINT
                  | CHECK_CONSTRAINT

PRIMARY_KEY_CONSTRAINT := "PRIMARY" + WS_REQ + "KEY" + WS + "(" + WS + IDENT_LIST + WS + ")"
UNIQUE_CONSTRAINT      := "UNIQUE" + WS + "(" + WS + IDENT_LIST + WS + ")"
FOREIGN_KEY_CONSTRAINT := "FOREIGN" + WS_REQ + "KEY" + WS + "(" + WS + IDENT_LIST + WS + ")" + WS_REQ + "REFERENCES" + WS_REQ + TABLE_NAME + WS + "(" + WS + IDENT_LIST + WS + ")"
CHECK_CONSTRAINT       := "CHECK" + WS + "(" + WS + PREDICATE + WS + ")"

CREATE_VIEW := ("MATERIALIZED" + WS_REQ)? + "VIEW" + WS_REQ + VIEW_NAME + WS_REQ + "AS" + WS_REQ + SELECT_QUERY

DROP := "DROP" + WS_REQ + (DROP_TABLE | DROP_VIEW)
DROP_TABLE := "TABLE" + WS_REQ + ("IF" + WS_REQ + "EXISTS" + WS_REQ)? + TABLE_NAME
DROP_VIEW  := "VIEW" + WS_REQ + ("IF" + WS_REQ + "EXISTS" + WS_REQ)? + VIEW_NAME

/* =========================================================================
 * 4. DML: SELECT & SET OPERATIONS
 * ========================================================================= */
SELECT_QUERY := BASIC_SELECT + (WS_REQ + SET_OP + WS_REQ + BASIC_SELECT)*

SET_OP := "UNION" + (WS_REQ + "ALL")?
        | "INTERSECT" + (WS_REQ + "ALL")?
        | "EXCEPT" + (WS_REQ + "ALL")?

BASIC_SELECT := "SELECT" + WS_REQ + ("DISTINCT" + WS_REQ)? + SELECT_ITEMS + WS_REQ + "FROM" + WS_REQ + TABLE_REF + (WS_REQ + WHERE_CLAUSE)? + (WS_REQ + GROUP_BY_CLAUSE)? + (WS_REQ + ORDER_BY_CLAUSE)?

SELECT_ITEMS := SELECT_ITEM + (WS + "," + WS + SELECT_ITEM)*
SELECT_ITEM  := EXPR + (WS_REQ + ("AS" + WS_REQ)? + IDENTIFIER)?

WHERE_CLAUSE := "WHERE" + WS_REQ + PREDICATE

GROUP_BY_CLAUSE := "GROUP" + WS_REQ + "BY" + WS_REQ + IDENT_LIST + (WS_REQ + HAVING_CLAUSE)?
HAVING_CLAUSE   := "HAVING" + WS_REQ + PREDICATE

ORDER_BY_CLAUSE := "ORDER" + WS_REQ + "BY" + WS_REQ + ORDER_ITEMS
ORDER_ITEMS     := ORDER_ITEM + (WS + "," + WS + ORDER_ITEM)*
ORDER_ITEM      := IDENTIFIER + (WS_REQ + ("ASC" | "DESC"))?

/* --- Table References & Joins --- */
TABLE_REF := SINGLE_TABLE_REF + (WS + JOIN_CLAUSE | WS + "," + WS + SINGLE_TABLE_REF)*

SINGLE_TABLE_REF := (TABLE_NAME + (WS_REQ + ("AS" + WS_REQ)? + IDENTIFIER)?)
                  | ("(" + WS + SELECT_QUERY + WS + ")" + WS_REQ + ("AS" + WS_REQ)? + IDENTIFIER)

JOIN_CLAUSE := CROSS_JOIN | QUALIFIED_JOIN
CROSS_JOIN  := "CROSS" + WS_REQ + "JOIN" + WS_REQ + SINGLE_TABLE_REF
QUALIFIED_JOIN := JOIN_TYPE + WS_REQ + SINGLE_TABLE_REF + WS_REQ + "ON" + WS_REQ + PREDICATE
JOIN_TYPE := ("INNER" + WS_REQ)? + "JOIN"
           | "LEFT" + (WS_REQ + "OUTER")? + WS_REQ + "JOIN"
           | "RIGHT" + (WS_REQ + "OUTER")? + WS_REQ + "JOIN"
           | "FULL" + (WS_REQ + "OUTER")? + WS_REQ + "JOIN"

/* =========================================================================
 * 5. DML: INSERT, UPDATE, DELETE
 * ========================================================================= */
INSERT := "INSERT" + WS_REQ + "INTO" + WS_REQ + TABLE_NAME + (WS + "(" + WS + IDENT_LIST + WS + ")")? + WS_REQ + (INSERT_VALUES | SELECT_QUERY)
INSERT_VALUES := "VALUES" + WS + ROW_VALUES + (WS + "," + WS + ROW_VALUES)*
ROW_VALUES    := "(" + WS + EXPR_LIST + WS + ")"

UPDATE := "UPDATE" + WS_REQ + TABLE_NAME + WS_REQ + "SET" + WS_REQ + ASSIGNMENTS + (WS_REQ + WHERE_CLAUSE)?
ASSIGNMENTS := ASSIGNMENT + (WS + "," + WS + ASSIGNMENT)*
ASSIGNMENT  := COLUMN_NAME + WS + "=" + WS + EXPR

DELETE := "DELETE" + WS_REQ + "FROM" + WS_REQ + TABLE_NAME + (WS_REQ + WHERE_CLAUSE)?

/* =========================================================================
 * 6. PREDICATES (LOGICAL / CONDITIONS)
 * ========================================================================= */
PREDICATE     := OR_PRED
OR_PRED       := AND_PRED + (WS_REQ + "OR" + WS_REQ + AND_PRED)*
AND_PRED      := NOT_PRED + (WS_REQ + "AND" + WS_REQ + NOT_PRED)*
NOT_PRED      := ("NOT" + WS_REQ)? + ATOMIC_PRED

ATOMIC_PRED := "TRUE"
             | "FALSE"
             | "(" + WS + PREDICATE + WS + ")"
             | "EXISTS" + WS + "(" + WS + SELECT_QUERY + WS + ")"
             | EXPR + WS_REQ + "IS" + WS_REQ + ("NOT" + WS_REQ)? + "NULL"
             | EXPR + WS_REQ + "LIKE" + WS_REQ + {string_literal}
             | EXPR + WS_REQ + "BETWEEN" + WS_REQ + EXPR + WS_REQ + "AND" + WS_REQ + EXPR
             | EXPR + WS_REQ + "IN" + WS + "(" + WS + (LITERAL_LIST | SELECT_QUERY) + WS + ")"
             | EXPR + WS + COMPARISON_OP + WS_REQ + ("ALL" | "ANY") + WS + "(" + WS + SELECT_QUERY + WS + ")"
             | EXPR + WS + COMPARISON_OP + WS + EXPR

COMPARISON_OP := "=" | "!=" | "<>" | ">=" | ">" | "<=" | "<"

/* =========================================================================
 * 7. EXPRESSIONS & ARITHMETIC
 * ========================================================================= */
EXPR         := ADD_SUB_EXPR
ADD_SUB_EXPR := MUL_DIV_EXPR + (WS + ("+" | "-") + WS + MUL_DIV_EXPR)*
MUL_DIV_EXPR := UNARY_EXPR + (WS + ("*" | "/") + WS + UNARY_EXPR)*
UNARY_EXPR   := ("-" + WS)? + PRIMARY_EXPR

PRIMARY_EXPR := LITERAL
              | AGGREGATE
              | FUNCTION_CALL
              | COLUMN_REF
              | "(" + WS + SELECT_QUERY + WS + ")"
              | "(" + WS + EXPR + WS + ")"

COLUMN_REF := (TABLE_NAME + "." + (COLUMN_NAME | "*"))
            | COLUMN_NAME
            | "*"

AGGREGATE := "COUNT" + WS + "(" + WS + "*" + WS + ")"
           | "COUNT" + WS + "(" + WS + ("DISTINCT" + WS_REQ)? + EXPR + WS + ")"
           | "SUM" + WS + "(" + WS + ("DISTINCT" + WS_REQ)? + EXPR + WS + ")"
           | "AVG" + WS + "(" + WS + ("DISTINCT" + WS_REQ)? + EXPR + WS + ")"
           | ("MIN" | "MAX") + WS + "(" + WS + EXPR + WS + ")"

FUNCTION_CALL := {function_name} + WS + "(" + WS + (EXPR_LIST)? + WS + ")"

/* =========================================================================
 * 8. LITERALS & LISTS
 * ========================================================================= */
LITERAL := {integer} | {float} | {string} | "TRUE" | "FALSE" | "NULL"

IDENT_LIST   := IDENTIFIER + (WS + "," + WS + IDENTIFIER)*
EXPR_LIST    := EXPR + (WS + "," + WS + EXPR)*
LITERAL_LIST := LITERAL + (WS + "," + WS + LITERAL)*
```

## Parser operations

Here's the pseudo-code:
```mermaid
railroad-ebnf-beta
    title Expression Grammar

    query = whitespace (select | insert | update | delete | create | drop) whitespace ";";
    
    %% done
    select = select_query | sub_select_query whitespace set_operation whitespace sub_select_query;
    
    select_query = "select" whitespace select_value (whitespace "," whitespace select_value)* (whitespace "from" whitespace table_select where_clause? group_by? order_by?)?;
    table_select = table_name whitespace identifier ("," table_name whitespace identifier)*;
    group_by = whitespace "group" whitespace "by" whitespace group_by whitespace (table_column | literal) (whitespace "," whitespace table_column | literal)* (whitespace "having" whitespace where_clause)?;
    order_by = "order" whitespace "by" whitespace ((table_column | literal) (whitespace ("asc" | "desc"))?)*;
    where_clause = whitespace "where" whitespace predicate;

    select_value = (literal | table_col_expor | aggregate "(" whitespace table_col_expr whitespace ")") (whitespace "as" whitespace literal whitespace)?;
    table_column = column_name | (table_name "." column_name);
    table_col_expr = table_column | ("(" table_col_expr ")") | table_col_expr ("+" | "*" | "-" | "/") table_col_expr;
    aggregate = "sum" | "count" | "max" | "min" | "avg" | "product";
    %% done
    sub_select_query = "(" whitespace select_query whitespace ")";
    %%done
    set_operation = ("union" | "intersect" | "except") (whitespace "all")?;

    %% done
    insert = "insert" whitespace "into" table_name whitespace (insert_values | sub_select_query);
    insert_values = "values" whitespace value_list (whitespace "," whitespace value_list)*;

    %% done
    update = "update" whitespace table_name whitespace "set" whitespace column_name whitespace "=" whitespace value where_clause?;

    %% done
    delete = "delete" whitespace "from" whitespace table_name whitespace where_clause?;

    %% done
    create = "create" whitespace (create_view | create_table);
    %% done
    create_view = "materialized"? whitespace "view" whitespace table_name whitespace "as" whitespace sub_select_query;
    
    create_table = "table" whitespace view_name;

    %% done
    drop = "drop" whitespace ("table" whitespace table_name | "view" whitespace view_name) ("if" whitespace "not" whitespace "exists")?;

    table_name = identifier;
    column_name = identifier;
    view_name = identifier;
    
    literal = string | integer | float | boolean | "null";
    value = literal | sub_select_query;
    value_list = "(" whitespace value (whitespace "," whitespace value)* whitespace ")";
```

