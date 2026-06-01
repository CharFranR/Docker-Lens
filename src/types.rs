pub struct DbData {
    pub port: String,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String
}

pub struct TablaInfo {
    pub nombre: String,
    pub columnas: Vec<ColumnaInfo>,
}

pub struct ColumnaInfo {
    pub nombre: String,
    pub tipo: String,       
    pub nullable: String,
    pub default: Option<String>,
}

// SQLite schema types

pub struct SQLiteSchema {
    pub tables: Vec<SQLiteTable>,
}

pub struct SQLiteTable {
    pub name: String,
    pub columns: Vec<SQLiteColumn>,
}

pub struct SQLiteColumn {
    pub name: String,
    pub sqlite_type: String,  // TEXT, INTEGER, REAL, BLOB
    pub nullable: bool,
    pub default: Option<String>,
}