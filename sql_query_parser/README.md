# SQL query parser

There are 2 stages to this:
1. Take the SQL string and feed it into a state machine lexer to grab the tokens from it and ensure that the SQL string is correct.
2. Run the list of tokens through a parser to generate the AST from this.

Design for this is [here](../docs/design/sql_query_parser.md).