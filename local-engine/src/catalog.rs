use crate::storage::StorageManager;

// TODO implement this catalog module for the local engine. This will involve implementing the catalog interface and providing methods for managing metadata about tables, indexes, and other database objects.

pub fn resolve_relation<SM: StorageManager>(transaction: &SM::Tx, relation_name: &str) {}

pub fn create_table<SM: StorageManager>(transaction: &SM::Tx, table_name: &str) {}

pub fn alter_table<SM: StorageManager>(transaction: &SM::Tx, table_name: &str) {}

pub fn drop_table<SM: StorageManager>(transaction: &SM::Tx, table_name: &str) {}

pub fn create_index<SM: StorageManager>(transaction: &SM::Tx, index_name: &str) {}

pub fn drop_index<SM: StorageManager>(transaction: &SM::Tx, index_name: &str) {}

pub fn create_view<SM: StorageManager>(transaction: &SM::Tx, view_name: &str) {}

pub fn replace_view<SM: StorageManager>(transaction: &SM::Tx, view_name: &str) {}

pub fn drop_view<SM: StorageManager>(transaction: &SM::Tx, view_name: &str) {}
