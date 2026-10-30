# SQL Query Lexer and Parser

## Grammar for Lexer

Here's the mermaid diagram:
```mermaid
railroad-ebnf-beta
    title "SQL Query Grammar"

    %% =========================================================================
    %% 1. TOP-LEVEL STATEMENTS
    %% =========================================================================
    query = [ whitespace ] ( select | insert | update | delete | create | drop ) [ whitespace ] [ ";" [ whitespace ] ] ;

    %% =========================================================================
    %% 2. DML: SELECT & SET OPERATIONS
    %% =========================================================================
    select = select_term { whitespace set_operation whitespace select_term } ;
    select_term = basic_select | sub_select_query ;
    sub_select_query = "(" [ whitespace ] select [ whitespace ] ")" ;

    set_operation = ( "union" | "intersect" | "except" ) [ whitespace "all" ] ;

    basic_select = "select" [ whitespace "distinct" ] whitespace select_items [ whitespace "from" whitespace table_ref ] [ where_clause ] [ group_by_clause ] [ order_by_clause ] ;

    select_items = select_item { [ whitespace ] "," [ whitespace ] select_item } ;
    select_item = expr [ whitespace [ "as" whitespace ] identifier ] ;

    %% =========================================================================
    %% 3. TABLE REFERENCES & JOINS
    %% =========================================================================
    table_ref = single_table_ref { [ whitespace ] ( join_clause | ( "," [ whitespace ] single_table_ref ) ) } ;

    single_table_ref = ( table_name | view_name | sub_select_query ) [ whitespace [ "as" whitespace ] identifier ] ;

    join_clause = cross_join | qualified_join ;
    cross_join = "cross" whitespace "join" whitespace single_table_ref ;
    qualified_join = join_type whitespace single_table_ref whitespace "on" whitespace predicate ;

    join_type = [ "inner" whitespace ] "join"
              | "left" [ whitespace "outer" ] whitespace "join"
              | "right" [ whitespace "outer" ] whitespace "join"
              | "full" [ whitespace "outer" ] whitespace "join" ;

    %% =========================================================================
    %% 4. CLAUSES: WHERE, GROUP BY, HAVING, ORDER BY
    %% =========================================================================
    where_clause = whitespace "where" whitespace predicate ;

    group_by_clause = whitespace "group" whitespace "by" whitespace ident_list [ whitespace having_clause ] ;
    having_clause = "having" whitespace predicate ;

    order_by_clause = whitespace "order" whitespace "by" whitespace order_items ;
    order_items = order_item { [ whitespace ] "," [ whitespace ] order_item } ;
    order_item = expr [ whitespace ( "asc" | "desc" ) ] ;

    %% =========================================================================
    %% 5. DML: INSERT, UPDATE, DELETE
    %% =========================================================================
    insert = "insert" whitespace "into" whitespace table_name [ [ whitespace ] "(" [ whitespace ] ident_list [ whitespace ] ")" ] whitespace ( insert_values | select ) ;
    insert_values = "values" [ whitespace ] row_values { [ whitespace ] "," [ whitespace ] row_values } ;
    row_values = "(" [ whitespace ] expr_list [ whitespace ] ")" ;

    update = "update" whitespace table_name whitespace "set" whitespace assignments [ where_clause ] ;
    assignments = assignment { [ whitespace ] "," [ whitespace ] assignment } ;
    assignment = column_name [ whitespace ] "=" [ whitespace ] expr ;

    delete = "delete" whitespace "from" whitespace table_name [ where_clause ] ;

    %% =========================================================================
    %% 6. DDL: CREATE & DROP
    %% =========================================================================
    create = "create" whitespace ( create_table | create_view ) ;

    create_table = "table" [ whitespace "if" whitespace "not" whitespace "exists" ] whitespace table_name [ whitespace ] "(" [ whitespace ] table_elements [ whitespace ] ")" ;
    table_elements = table_element { [ whitespace ] "," [ whitespace ] table_element } ;
    table_element = column_def | table_constraint ;

    column_def = column_name whitespace data_type [ whitespace "not" whitespace "null" ] [ whitespace "default" whitespace literal ] ;

    data_type = "int"
              | "float"
              | "date"
              | "time"
              | ( "char" [ whitespace ] "(" [ whitespace ] integer [ whitespace ] ")" )
              | ( "varchar" [ whitespace ] "(" [ whitespace ] integer [ whitespace ] ")" ) ;

    table_constraint = primary_key_constraint
                     | unique_constraint
                     | foreign_key_constraint
                     | check_constraint ;

    primary_key_constraint = "primary" whitespace "key" [ whitespace ] "(" [ whitespace ] ident_list [ whitespace ] ")" ;
    unique_constraint = "unique" [ whitespace ] "(" [ whitespace ] ident_list [ whitespace ] ")" ;
    foreign_key_constraint = "foreign" whitespace "key" [ whitespace ] "(" [ whitespace ] ident_list [ whitespace ] ")" whitespace "references" whitespace table_name [ whitespace ] "(" [ whitespace ] ident_list [ whitespace ] ")" ;
    check_constraint = "check" [ whitespace ] "(" [ whitespace ] predicate [ whitespace ] ")" ;

    create_view = [ "materialized" whitespace ] "view" whitespace view_name whitespace "as" whitespace ( select | sub_select_query ) ;

    drop = "drop" whitespace ( drop_table | drop_view ) ;
    drop_table = "table" [ whitespace "if" whitespace "exists" ] whitespace table_name ;
    drop_view = "view" [ whitespace "if" whitespace "exists" ] whitespace view_name ;

    %% =========================================================================
    %% 7. PREDICATES (CONDITIONS)
    %% =========================================================================
    predicate = or_pred ;
    or_pred = and_pred { whitespace "or" whitespace and_pred } ;
    and_pred = not_pred { whitespace "and" whitespace not_pred } ;
    not_pred = [ "not" whitespace ] atomic_pred ;

    atomic_pred = boolean
                | ( "(" [ whitespace ] predicate [ whitespace ] ")" )
                | ( "exists" [ whitespace ] sub_select_query )
                | ( expr whitespace "is" [ whitespace "not" ] whitespace "null" )
                | ( expr whitespace "like" whitespace string )
                | ( expr whitespace "between" whitespace expr whitespace "and" whitespace expr )
                | ( expr whitespace "in" [ whitespace ] "(" [ whitespace ] ( literal_list | select ) [ whitespace ] ")" )
                | ( expr [ whitespace ] comparison_op [ whitespace ( "all" | "any" ) ] [ whitespace ] ( sub_select_query | expr ) ) ;

    comparison_op = "=" | "!=" | "<>" | ">=" | ">" | "<=" | "<" ;

    %% =========================================================================
    %% 8. EXPRESSIONS & ARITHMETIC
    %% =========================================================================
    expr = add_sub_expr ;
    add_sub_expr = mul_div_expr { [ whitespace ] ( "+" | "-" ) [ whitespace ] mul_div_expr } ;
    mul_div_expr = unary_expr { [ whitespace ] ( "*" | "/" ) [ whitespace ] unary_expr } ;
    unary_expr = [ "-" [ whitespace ] ] primary_expr ;

    primary_expr = literal
                 | column_ref
                 | aggregate
                 | function_call
                 | sub_select_query
                 | ( "(" [ whitespace ] expr [ whitespace ] ")" ) ;

    column_ref = [ table_name "." ] ( column_name | "*" ) ;

    aggregate = ( "count" [ whitespace ] "(" [ whitespace ] ( "*" | ( [ "distinct" whitespace ] expr ) ) [ whitespace ] ")" )
              | ( ( "sum" | "avg" ) [ whitespace ] "(" [ whitespace ] [ "distinct" whitespace ] expr [ whitespace ] ")" )
              | ( ( "min" | "max" | "product" ) [ whitespace ] "(" [ whitespace ] expr [ whitespace ] ")" ) ;

    function_call = identifier [ whitespace ] "(" [ whitespace ] [ expr_list ] [ whitespace ] ")" ;

    %% =========================================================================
    %% 9. LITERALS, IDENTIFIERS & LISTS
    %% =========================================================================
    literal = string | number | boolean | "null" ;
    number = integer | float ;
    boolean = "true" | "false" ;

    ident_list = identifier { [ whitespace ] "," [ whitespace ] identifier } ;
    expr_list = expr { [ whitespace ] "," [ whitespace ] expr } ;
    literal_list = literal { [ whitespace ] "," [ whitespace ] literal } ;

    table_name = identifier ;
    column_name = identifier ;
    view_name = identifier ;
```

