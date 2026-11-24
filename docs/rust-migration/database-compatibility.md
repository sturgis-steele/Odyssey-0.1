# SQLite compatibility

The Python migration and a read-only legacy validation database agree on user_version 1. That database contained 14 conversations and 19 turns and passed integrity_check. This inspection used no personal conversation content.

| Object | Storage contract |
| --- | --- |
| conversations | id TEXT PRIMARY KEY, title/created_at/updated_at TEXT NOT NULL. UUID text and UTC ISO 8601 timestamps with six fractional digits and +00:00. |
| turns | seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, created_at/user TEXT NOT NULL, assistant TEXT NOT NULL DEFAULT '', status TEXT NOT NULL constrained to generating/complete/interrupted/error, error nullable TEXT, activity/transcript TEXT NOT NULL DEFAULT '[]'. |
| Indexes | conversations_recent(updated_at DESC,id), turns_conversation(conversation_id,seq), unique one_generation(status) filtered to generating, plus SQLite's implicit keys. |

Activity is a JSON array of public tool outcomes. Transcript is an ordered JSON message array. Assistant function calls carry IDs and JSON argument strings. Tool messages carry matching tool_call_id, name and serialized outcome content. Private reasoning never enters storage. Only complete transcripts enter subsequent prompts.

Rust uses the same schema and sidecar protocol. Version zero is created atomically. A newer schema is refused. No v1 data conversion is necessary. Both implementations acquire BEGIN EXCLUSIVE on a separate `.lock` SQLite file before recovering orphan turns. A held owner lock prevents the second process from modifying or recovering a live turn. Foreign keys, a one-second data busy timeout, WAL, FULL synchronization and immediate write transactions protect snapshots. The database worker serializes operations away from Tokio's request threads.

To test an existing installation, close all writers or use SQLite's backup API into a new directory. A file copy of only the main database while WAL is active can omit committed data. Point DATABASE_PATH at the backup. Compare row counts, all stored fields, foreign_key_check and integrity_check. Roll back by stopping the Rust process and pointing the legacy launcher at the same compatible test copy or restoring the backup. Do not overwrite a live database.
