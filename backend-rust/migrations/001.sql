CREATE TABLE conversations (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX conversations_recent ON conversations(updated_at DESC, id);
CREATE TABLE turns (
    seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL, user TEXT NOT NULL, assistant TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK(status IN ('generating','complete','interrupted','error')),
    error TEXT, activity TEXT NOT NULL DEFAULT '[]', transcript TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX turns_conversation ON turns(conversation_id, seq);
CREATE UNIQUE INDEX one_generation ON turns(status) WHERE status='generating';
