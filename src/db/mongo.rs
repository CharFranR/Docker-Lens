// MongoDB adapter via mongodb crate (async).
// Uses tokio runtime internally to bridge async → sync for the dispatch layer.
use std::io::{Error, ErrorKind};

use futures::StreamExt;
use mongodb::bson::{self, doc, Document};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection};
use serde_json::Value;

use crate::types::{GenericCredentials, TablaInfo, ColumnaInfo};

const DEFAULT_PORT: &str = "27017";

/// Build a MongoDB connection URI from credentials.
fn build_uri(creds: &GenericCredentials) -> String {
    let host = if creds.host.is_empty() {
        "localhost"
    } else {
        &creds.host
    };
    let port = if creds.port.is_empty() {
        DEFAULT_PORT
    } else {
        &creds.port
    };

    if creds.user.is_empty() {
        format!("mongodb://{host}:{port}/")
    } else {
        format!(
            "mongodb://{user}:{password}@{host}:{port}/",
            user = creds.user,
            password = creds.password,
        )
    }
}

/// Get the database name from credentials.
fn db_name(creds: &GenericCredentials) -> &str {
    if creds.database.is_empty() {
        "admin"
    } else {
        &creds.database
    }
}

/// List all collections in the database.
pub fn list_tables(creds: &GenericCredentials) -> std::io::Result<String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    let dbn = db_name(creds).to_string();

    let uri = build_uri(creds);
    rt.block_on(async {
        let client = Client::with_uri_str(&uri).await.map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB connect: {e}")))?;
        let db = client.database(&dbn);
        let names = db
            .list_collection_names(None)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB list: {e}")))?;

        if names.is_empty() {
            Ok(String::from("(No collections found)"))
        } else {
            Ok(names.join("\n"))
        }
    })
}

/// Execute a MongoDB query. The query must be a JSON document with one of:
///
/// ```json
/// {"listCollections": true}
/// {"find": "collection", "filter": {}, "projection": {}, "limit": 10}
/// {"aggregate": "collection", "pipeline": [{"$match": {}}]}
/// ```
pub fn make_query(creds: &GenericCredentials, query: &str) -> std::io::Result<String> {

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    let dbn = db_name(creds).to_string();

    // Parse the JSON query
    let q: Value = serde_json::from_str(query).map_err(|e| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("MongoDB query must be valid JSON: {e}"),
        )
    })?;

    let uri = build_uri(creds);
    rt.block_on(async {
        let client = Client::with_uri_str(&uri).await.map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB connect: {e}")))?;
        let db = client.database(&dbn);

        // Determine action from JSON fields
        if q.get("listCollections").is_some() {
            let names = db
                .list_collection_names(None)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB: {e}")))?;
            return Ok(names.join("\n"));
        }

        if let Some(collection_name) = q.get("find").and_then(|v| v.as_str()) {
            let coll: Collection<Document> = db.collection(collection_name);
            let filter = q
                .get("filter")
                .and_then(|v| bson::to_document(v).ok())
                .unwrap_or_else(|| doc! {});

            let projection = q
                .get("projection")
                .and_then(|v| bson::to_document(v).ok());

            let limit = q.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);

            let mut find_opts = FindOptions::default();
            if let Some(proj) = projection {
                find_opts.projection = Some(proj);
            }
            find_opts.limit = Some(limit);

            let mut cursor = coll
                .find(filter, find_opts)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB find: {e}")))?;

            let mut results: Vec<Document> = Vec::new();
            while let Some(doc_result) = cursor.next().await {
                let doc = doc_result
                    .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB cursor: {e}")))?;
                results.push(doc);
            }

            return docs_to_string(&results);
        }

        if let Some(collection_name) = q.get("aggregate").and_then(|v| v.as_str()) {
            let coll: Collection<Document> = db.collection(collection_name);
            let pipeline: Vec<Document> = q
                .get("pipeline")
                .and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|item| bson::to_document(item).ok())
                            .collect()
                    })
                })
                .unwrap_or_default();

            let mut cursor = coll
                .aggregate(pipeline, None)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB aggregate: {e}")))?;

            let mut results: Vec<Document> = Vec::new();
            while let Some(doc_result) = cursor.next().await {
                let doc = doc_result
                    .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB cursor: {e}")))?;
                results.push(doc);
            }

            return docs_to_string(&results);
        }

        Err(Error::new(
            ErrorKind::InvalidInput,
            "MongoDB query must have 'find', 'aggregate', or 'listCollections' field",
        ))
    })
}

/// Export a MongoDB collection to CSV.
pub fn export_csv(
    creds: &GenericCredentials,
    collection: &str,
    file_path: &str,
) -> std::io::Result<()> {
    
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    let dbn = db_name(creds).to_string();
    let coll_name = collection.to_string();

     let uri = build_uri(creds);
    rt.block_on(async {

        let client = Client::with_uri_str(&uri).await.map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB connect: {e}")))?;

        let db = client.database(&dbn);
        let coll: Collection<Document> = db.collection(&coll_name);

        let mut cursor = coll
            .find(doc! {}, None)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB find: {e}")))?;

        let mut docs: Vec<Document> = Vec::new();
        while let Some(doc_result) = cursor.next().await {
            let doc = doc_result
                .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB cursor: {e}")))?;
            docs.push(doc);
        }

        if docs.is_empty() {
            std::fs::write(file_path, "")
                .map_err(|e| Error::new(ErrorKind::Other, format!("Write CSV: {e}")))?;
            return Ok(());
        }

        // Collect all unique keys across all documents
        let mut all_keys: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for doc in &docs {
            for key in doc.keys() {
                if seen.insert(key.clone()) {
                    all_keys.push(key.clone());
                }
            }
        }

        // Write CSV
        let mut wtr = csv::Writer::from_path(file_path)
            .map_err(|e| Error::new(ErrorKind::Other, format!("CSV writer: {e}")))?;

        // Headers
        wtr.write_record(&all_keys)
            .map_err(|e| Error::new(ErrorKind::Other, format!("CSV headers: {e}")))?;

        // Rows
        for doc in &docs {
            let row: Vec<String> = all_keys
                .iter()
                .map(|key| {
                    doc.get(key.as_str())
                        .map(bson_value_to_string)
                        .unwrap_or_default()
                })
                .collect();
            wtr.write_record(&row)
                .map_err(|e| Error::new(ErrorKind::Other, format!("CSV row: {e}")))?;
        }

        wtr.flush()
            .map_err(|e| Error::new(ErrorKind::Other, format!("CSV flush: {e}")))?;

        Ok(())
    })
}

/// Format documents as a tab-separated table.
fn docs_to_string(docs: &[Document]) -> std::io::Result<String> {
    if docs.is_empty() {
        return Ok(String::from("(No documents found)"));
    }

    // Collect all keys
    let mut all_keys: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for doc in docs {
        for key in doc.keys() {
            if seen.insert(key.clone()) {
                all_keys.push(key.clone());
            }
        }
    }

    // Build tab-separated table
    let mut output = String::new();

    // Headers
    output.push_str(&all_keys.join("\t"));
    output.push('\n');

    // Separator
    let sep: Vec<&str> = all_keys.iter().map(|_| "---").collect();
    output.push_str(&sep.join("\t"));
    output.push('\n');

    // Rows
    for doc in docs {
        let row: Vec<String> = all_keys
            .iter()
            .map(|key| {
                doc.get(key.as_str())
                    .map(bson_value_to_string)
                    .unwrap_or_default()
            })
            .collect();
        output.push_str(&row.join("\t"));
        output.push('\n');
    }

    Ok(output)
}

/// Convert a BSON value to a display string.
fn bson_value_to_string(value: &bson::Bson) -> String {
    match value {
        bson::Bson::String(s) => s.clone(),
        bson::Bson::Int32(i) => i.to_string(),
        bson::Bson::Int64(i) => i.to_string(),
        bson::Bson::Double(f) => f.to_string(),
        bson::Bson::Boolean(b) => b.to_string(),
        bson::Bson::ObjectId(oid) => oid.to_hex(),
        bson::Bson::DateTime(dt) => dt.to_string(),
        bson::Bson::Null => "null".to_string(),
        bson::Bson::Array(arr) => {
            let items: Vec<String> = arr.iter().map(bson_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        bson::Bson::Document(doc) => {
            let items: Vec<String> = doc
                .iter()
                .map(|(k, v)| format!("{k}: {}", bson_value_to_string(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        _ => format!("{value:?}"),
    }
}

/// MongoDB schema inspection by sampling first document from each collection.
pub fn inspect_schema_mongo(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    // Get collection names
    let coll_names_raw = list_tables(creds)?;
    if coll_names_raw.contains("No collections found") || coll_names_raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let coll_names: Vec<String> = coll_names_raw.lines().map(|l| l.trim().to_string()).collect();

    let mut tables = Vec::new();

    for coll_name in &coll_names {
        // Query one document to infer fields
        let query = format!(
            r#"{{"find": "{}", "limit": 1}}"#,
            coll_name
        );
        let result = make_query(creds, &query)?;

        if result.contains("No documents found") || result.trim().is_empty() {
            tables.push(TablaInfo {
                nombre: coll_name.clone(),
                columnas: Vec::new(),
            });
            continue;
        }

        // Parse the tab-separated output to infer column types
        let columns = parse_mongo_fields_from_output(&result);
        tables.push(TablaInfo {
            nombre: coll_name.clone(),
            columnas: columns,
        });
    }

    Ok(tables)
}

/// Parse MongoDB docs_to_string output to infer field names and types.
fn parse_mongo_fields_from_output(output: &str) -> Vec<ColumnaInfo> {
    let mut lines = output.lines();
    let header_line = lines.next().unwrap_or("");
    // Skip separator line
    let _sep = lines.next();
    let first_row = lines.next().unwrap_or("");

    let headers: Vec<&str> = header_line.split('\t').map(|s| s.trim()).collect();
    let values: Vec<&str> = first_row.split('\t').map(|s| s.trim()).collect();

    headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let val = values.get(i).copied().unwrap_or("");
            let inferred_type = infer_mongo_type(val);
            ColumnaInfo {
                nombre: name.to_string(),
                tipo: inferred_type,
                nullable: "YES".to_string(),
                default: None,
            }
        })
        .collect()
}

/// Infer a SQL-like type from a MongoDB value string.
fn infer_mongo_type(val: &str) -> String {
    if val.is_empty() || val == "null" {
        return "text".to_string();
    }
    if val == "true" || val == "false" {
        return "boolean".to_string();
    }
    if val.parse::<i64>().is_ok() {
        return "integer".to_string();
    }
    if val.parse::<f64>().is_ok() {
        return "double".to_string();
    }
    "text".to_string()
}


/// Export MongoDB to SQLite by sampling each collection.
pub fn export_mongo_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    let coll_names_raw = list_tables(creds)?;
    if coll_names_raw.contains("No collections found") || coll_names_raw.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "No collections found in MongoDB",
        ));
    }

    let coll_names: Vec<String> = coll_names_raw.lines().map(|l| l.trim().to_string()).collect();

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Error creating SQLite: {e}")))?;

    for coll_name in &coll_names {
        let temp_csv = format!("/tmp/dl_mongo_export_{}.csv", coll_name.replace(' ', "_"));
        export_csv(creds, coll_name, &temp_csv)?;

        // Read CSV to infer schema and create table
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&temp_csv)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error reading CSV for '{}': {e}", coll_name),
                )
            })?;

        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error reading headers: {e}")))?
            .iter()
            .map(|h| h.to_string())
            .collect();

        if headers.is_empty() {
            let _ = std::fs::remove_file(&temp_csv);
            continue;
        }

        // Create table with all TEXT columns (safe for MongoDB's flexible schema)
        let col_defs: Vec<String> = headers
            .iter()
            .map(|h| format!("\"{}\" TEXT", h))
            .collect();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (\n    {}\n);",
            coll_name,
            col_defs.join(",\n    ")
        );

        conn.execute_batch(&create_sql).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Error creating table '{}': {e}", coll_name),
            )
        })?;

        let placeholders: Vec<String> = (0..headers.len()).map(|_| "?".to_string()).collect();
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            coll_name,
            headers
                .iter()
                .map(|h| format!("\"{}\"", h))
                .collect::<Vec<_>>()
                .join(", "),
            placeholders.join(", ")
        );

        let tx = conn.unchecked_transaction().map_err(|e| {
            Error::new(ErrorKind::Other, format!("Error starting transaction: {e}"))
        })?;

        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error preparing INSERT for '{}': {e}", coll_name),
                )
            })?;

            for result in rdr.records() {
                let record = result.map_err(|e| {
                    Error::new(ErrorKind::Other, format!("Error reading record: {e}"))
                })?;
                let values: Vec<&str> = record.iter().collect();
                stmt.execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Error inserting into '{}': {e}", coll_name),
                        )
                    })?;
            }
        }

        tx.commit()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error on commit: {e}")))?;

        let _ = std::fs::remove_file(&temp_csv);
    }

    Ok(())
}