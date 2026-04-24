use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use directories::ProjectDirs;
use rusqlite::{Connection, params};

#[derive(Debug, Clone, Default)]
pub struct PersistedState {
    pub workspace_root: Option<PathBuf>,
    pub expanded_paths: BTreeSet<PathBuf>,
}

#[derive(Debug)]
pub struct Persistence {
    connection: Mutex<Connection>,
}

impl Persistence {
    pub fn new() -> Result<Self, String> {
        let project_dirs = ProjectDirs::from("com", "OfflineFirst", "PhaseOneExplorer")
            .ok_or_else(|| "failed to resolve app-local data directory".to_string())?;

        fs::create_dir_all(project_dirs.data_local_dir()).map_err(|err| {
            format!(
                "failed to create local app data directory {}: {err}",
                project_dirs.data_local_dir().display()
            )
        })?;

        let database_path = project_dirs.data_local_dir().join("state.sqlite3");
        let connection = Connection::open(&database_path).map_err(|err| {
            format!(
                "failed to open sqlite database {}: {err}",
                database_path.display()
            )
        })?;

        initialize_schema(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn load_state(&self) -> Result<PersistedState, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "failed to lock sqlite connection".to_string())?;

        let workspace_root = connection
            .query_row(
                "SELECT workspace_root FROM app_state WHERE id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None)
            .map(PathBuf::from);

        let mut expanded_paths = BTreeSet::new();
        let mut statement = connection
            .prepare("SELECT path FROM expanded_nodes ORDER BY path")
            .map_err(|err| format!("failed to prepare expanded_nodes query: {err}"))?;

        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|err| format!("failed to query expanded_nodes: {err}"))?;

        for row in rows {
            let path = row.map_err(|err| format!("failed to read expanded_nodes row: {err}"))?;
            expanded_paths.insert(PathBuf::from(path));
        }

        Ok(PersistedState {
            workspace_root,
            expanded_paths,
        })
    }

    pub fn save_state(&self, state: &PersistedState) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "failed to lock sqlite connection".to_string())?;

        let transaction = connection
            .transaction()
            .map_err(|err| format!("failed to open sqlite transaction: {err}"))?;

        let workspace_root = state
            .workspace_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());

        transaction
            .execute(
                "INSERT INTO app_state (id, workspace_root) VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET workspace_root = excluded.workspace_root",
                params![workspace_root],
            )
            .map_err(|err| format!("failed to upsert app_state: {err}"))?;

        transaction
            .execute("DELETE FROM expanded_nodes", [])
            .map_err(|err| format!("failed to clear expanded_nodes: {err}"))?;

        {
            let mut insert_statement = transaction
                .prepare("INSERT INTO expanded_nodes(path) VALUES (?1)")
                .map_err(|err| format!("failed to prepare expanded_nodes insert: {err}"))?;

            for path in &state.expanded_paths {
                let path_text = path.to_string_lossy().to_string();
                insert_statement
                    .execute(params![path_text])
                    .map_err(|err| format!("failed to insert expanded path: {err}"))?;
            }
        }

        transaction
            .commit()
            .map_err(|err| format!("failed to commit sqlite transaction: {err}"))?;

        Ok(())
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                workspace_root TEXT
            );

            CREATE TABLE IF NOT EXISTS expanded_nodes (
                path TEXT PRIMARY KEY
            );
            ",
        )
        .map_err(|err| format!("failed to initialize sqlite schema: {err}"))?;

    Ok(())
}
