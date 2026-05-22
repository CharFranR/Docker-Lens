use std::string;

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