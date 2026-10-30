use crate::expression::Literal;

#[derive(Debug, PartialEq, Clone)]
pub enum TupleEntryDataType {
    Char(u64),
    VarChar(u64),
    Float,
    Int,
    Date,
    Time,
}

#[derive(Debug, PartialEq, Clone)]
pub enum KeyConstraint {
    Unique(Vec<String>),
    PrimaryKey(Vec<String>),
    ForeignKey {
        entry: Vec<String>,
        referenced_table: String,
        referenced_entries: Vec<String>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct TupleSchemaEntry {
    pub entry: String,
    pub data_type: TupleEntryDataType,
    pub not_null: bool,
    pub default: Option<Literal>,
}

/// One schema change per ALTER TABLE statement.
#[derive(Debug, PartialEq, Clone)]
pub enum AlterTableAction {
    RenameTo(String),
    AddColumn(TupleSchemaEntry),
    DropColumn(String),
    RenameColumn {
        name: String,
        new_name: String,
    },
    AlterColumn {
        name: String,
        action: AlterColumnAction,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum AlterColumnAction {
    SetDataType(TupleEntryDataType),
    SetDefault(Literal),
    DropDefault,
    SetNotNull,
    DropNotNull,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AlterViewAction {
    RenameTo(String),
    As(Box<crate::dml::select::SqlQuery>),
}
