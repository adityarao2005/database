
pub enum TupleEntryDataType {
    Char(u64),
    VarChar(u64),
    Float,
    Int,
    Date,
    Time,
}

pub enum KeyConstraint {
    Unique(Vec<String>),
    PrimaryKey(Vec<String>),
    ForeignKey {
        entry: Vec<String>,
        referenced_table: String,
        referenced_entries: Vec<String>,
    },
}

pub struct TupleSchemaEntry {
    pub entry: String,
    pub data_type: TupleEntryDataType,
    pub not_null: bool,
}