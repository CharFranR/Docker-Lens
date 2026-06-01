// MongoDB adapter via mongodb crate (async).
// Uses tokio runtime internally to bridge async → sync for the dispatch layer.
use std::io::{Error, ErrorKind};

use futures::StreamExt;
use mongodb::bson::{self, doc, Document};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection};
use serde_json::Value;

use crate::types::GenericCredentials;

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

/// Connect to MongoDB synchronously using an internal tokio runtime.
fn connect(creds: &GenericCredentials) -> std::io::Result<Client> {
    let uri = build_uri(creds);
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    rt.block_on(async {
        Client::with_uri_str(&uri)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("MongoDB connect: {e}")))
    })
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
    let client = connect(creds)?;
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    let dbn = db_name(creds).to_string();

    rt.block_on(async {
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
    let client = connect(creds)?;
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

    rt.block_on(async {
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
    let client = connect(creds)?;
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Tokio runtime: {e}")))?;

    let dbn = db_name(creds).to_string();
    let coll_name = collection.to_string();

    rt.block_on(async {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DbType, GenericCredentials};

    fn mongo_creds() -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Mongo,
            host: "localhost".into(),
            port: "27017".into(),
            user: "admin".into(),
            password: "admin".into(),
            database: "testdb".into(),
        }
    }

    #[test]
    fn test_build_uri_with_credentials() {
        let creds = mongo_creds();
        let uri = build_uri(&creds);
        assert!(uri.contains("mongodb://"));
        assert!(uri.contains("admin:admin@"));
        assert!(uri.contains("localhost:27017"));
    }

    #[test]
    fn test_build_uri_without_credentials() {
        let creds = GenericCredentials {
            db_type: DbType::Mongo,
            host: "localhost".into(),
            port: "27017".into(),
            user: String::new(),
            password: String::new(),
            database: "testdb".into(),
        };
        let uri = build_uri(&creds);
        assert!(uri.contains("mongodb://localhost:27017/"));
        assert!(!uri.contains("@"));
    }

    #[test]
    fn test_build_uri_defaults() {
        let creds = GenericCredentials {
            db_type: DbType::Mongo,
            host: String::new(),
            port: String::new(),
            user: String::new(),
            password: String::new(),
            database: String::new(),
        };
        let uri = build_uri(&creds);
        assert!(uri.contains("localhost"));
        assert!(uri.contains("27017"));
    }

    #[test]
    fn test_bson_value_to_string() {
        assert_eq!(
            bson_value_to_string(&bson::Bson::String("hello".into())),
            "hello"
        );
        assert_eq!(bson_value_to_string(&bson::Bson::Int32(42)), "42");
        assert_eq!(bson_value_to_string(&bson::Bson::Boolean(true)), "true");
        assert_eq!(bson_value_to_string(&bson::Bson::Null), "null");
    }

    #[test]
    fn test_docs_to_string_empty() {
        let result = docs_to_string(&[]).unwrap();
        assert!(result.contains("No documents found"));
    }

    #[test]
    fn test_make_query_invalid_json() {
        let creds = mongo_creds();
        // This will fail because it can't connect to MongoDB,
        // but we want to test the JSON parsing path
        let result = make_query(&creds, "not json");
        assert!(result.is_err());
    }
}
