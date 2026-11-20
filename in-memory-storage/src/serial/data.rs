use std::collections::HashMap;

use engine::helpers::Value;
use local_engine::storage::{StorageManager, schema::constraints::ConstraintKind};

use crate::serial::SerialStorageManager;

type TableId = <SerialStorageManager as StorageManager>::TableId;
type IndexId = <SerialStorageManager as StorageManager>::IndexId;
type RowId = <SerialStorageManager as StorageManager>::RowId;
type ViewId = <SerialStorageManager as StorageManager>::ViewId;
type ColumnId = <SerialStorageManager as StorageManager>::ColumnId;
type ConstraintId = <SerialStorageManager as StorageManager>::ConstraintId;

#[derive(Clone, Debug, PartialEq)]
pub enum Constraint {
    PrimaryKey {
        columns: Vec<ColumnId>,
    },
    Unique {
        columns: Vec<ColumnId>,
    },
}

impl Constraint {

    pub fn kind(&self) -> ConstraintKind<SerialStorageManager> {
        match self {
            Constraint::PrimaryKey { columns } => ConstraintKind::PrimaryKey { columns: columns.clone() },
            Constraint::Unique { columns } => ConstraintKind::Unique { columns: columns.clone() },
        }
    }

    pub fn from_kind(kind: ConstraintKind<SerialStorageManager>) -> Result<Self, String> {
        match kind {
            ConstraintKind::PrimaryKey { columns } => Ok(Constraint::PrimaryKey { columns }),
            ConstraintKind::Unique { columns } => Ok(Constraint::Unique { columns }),
            _ => Err("Unsupported constraint kind".to_string()),

        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerialSchema {
    pub tables: HashMap<TableId, SerialTable>,
}

impl SerialSchema {
    pub fn new() -> Self {
        SerialSchema {
            tables: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerialTable {
    pub id: TableId,
    pub name: String,
    pub columns: HashMap<ColumnId, SerialColumn>,
    pub constraints: HashMap<ConstraintId, SerialConstraint>,
    pub rows: HashMap<RowId, Vec<Value>>,
    pub current_row_id: RowId,
    pub current_column_id: ColumnId,
    pub current_constraint_id: ConstraintId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerialColumn {
    pub id: ColumnId,
    pub name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerialConstraint {
    pub id: ConstraintId,
    pub constraint: Constraint,
}

