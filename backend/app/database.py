"""Turn snapshots commit atomically. Only complete turns enter model history."""
import json
import sqlite3
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

from .contracts import PublicError

SCHEMA = """
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
"""


def now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="microseconds")


class Store:
    def __init__(self, path: Path):
        self.path = path

    @contextmanager
    def ownership(self):
        """An OS-released SQLite lock prevents a second worker recovering live turns."""
        self.path.parent.mkdir(parents=True, exist_ok=True)
        owner = sqlite3.connect(str(self.path) + ".lock", timeout=0)
        try:
            owner.execute("BEGIN EXCLUSIVE")
            self.initialize()
            yield
        finally:
            owner.close()

    @contextmanager
    def connection(self, write=False):
        db = sqlite3.connect(self.path, timeout=1)
        try:
            db.row_factory = sqlite3.Row
            db.execute("PRAGMA foreign_keys=ON")
            if write:
                db.execute("BEGIN IMMEDIATE")
            with db:
                yield db
        finally:
            db.close()

    def initialize(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.connection() as db:
            db.execute("PRAGMA journal_mode=WAL")
            db.execute("BEGIN IMMEDIATE")
            version = db.execute("PRAGMA user_version").fetchone()[0]
            if version > 1:
                raise RuntimeError("The database requires a newer version of Odyssey.")
            if version == 0:
                for statement in SCHEMA.split(";"):
                    if statement.strip():
                        db.execute(statement)
                db.execute("PRAGMA user_version=1")
            db.execute("UPDATE turns SET status='interrupted', error=? WHERE status='generating'",
                       ("Generation was interrupted. Send another message to continue.",))

    @staticmethod
    def get(db, cid: str) -> dict:
        row = db.execute("SELECT * FROM conversations WHERE id=?", (cid,)).fetchone()
        if not row:
            raise PublicError("not_found", "Conversation could not be found.", 404)
        return dict(row)

    def create(self) -> dict:
        cid, stamp = str(uuid4()), now()
        with self.connection(write=True) as db:
            db.execute("INSERT INTO conversations VALUES (?,?,?,?)", (cid, "New conversation", stamp, stamp))
            return self.get(db, cid)

    def listing(self, limit: int, offset: int) -> list[dict]:
        with self.connection() as db:
            return [dict(r) for r in db.execute(
                "SELECT * FROM conversations ORDER BY updated_at DESC,id LIMIT ? OFFSET ?", (limit, offset))]

    def detail(self, cid: str, limit: int, before: int | None) -> dict:
        with self.connection() as db:
            conversation = self.get(db, cid)
            rows = db.execute("""
                SELECT seq,id,created_at,user,assistant,status,error,activity FROM turns
                WHERE conversation_id=? AND seq<? ORDER BY seq DESC LIMIT ?
                """, (cid, before or 2**63-1, limit + 1)).fetchall()
        turns = []
        for row in reversed(rows[:limit]):
            turn = dict(row)
            turn["tools"] = json.loads(turn.pop("activity"))
            turns.append(turn)
        return {**conversation, "turns": turns,
                "next_before": turns[0]["seq"] if len(rows) > limit else None}

    def delete(self, cid: str):
        with self.connection(write=True) as db:
            self.get(db, cid)
            if db.execute("SELECT 1 FROM turns WHERE conversation_id=? AND status='generating'", (cid,)).fetchone():
                raise PublicError("busy", "Stop the active response before deleting this conversation.", 409)
            db.execute("DELETE FROM conversations WHERE id=?", (cid,))

    def begin(self, cid: str, user: str) -> str:
        tid, stamp = str(uuid4()), now()
        with self.connection(write=True) as db:
            self.get(db, cid)
            if db.execute("SELECT 1 FROM turns WHERE status='generating'").fetchone():
                raise PublicError("busy", "A response is already running on this device.", 409)
            first = not db.execute("SELECT 1 FROM turns WHERE conversation_id=? LIMIT 1", (cid,)).fetchone()
            db.execute("""INSERT INTO turns (id,conversation_id,created_at,user,status)
                          VALUES (?,?,?,?,'generating')""", (tid, cid, stamp, user))
            db.execute("UPDATE conversations SET updated_at=? WHERE id=?", (stamp, cid))
            if first:
                title = " ".join(user.split())
                db.execute("UPDATE conversations SET title=? WHERE id=?", (title[:68], cid))
        return tid

    def history(self, cid: str, limit: int) -> list[list[dict]]:
        with self.connection() as db:
            rows = db.execute("""SELECT transcript FROM turns WHERE conversation_id=? AND status='complete'
                                 ORDER BY seq DESC LIMIT ?""", (cid, limit)).fetchall()
        return [json.loads(row[0]) for row in reversed(rows)]

    def checkpoint(self, tid: str, text: str, tools: list, transcript: list, status: str, error=None):
        with self.connection(write=True) as db:
            db.execute("""UPDATE turns SET assistant=?,activity=?,transcript=?,status=?,error=?
                          WHERE id=? AND status='generating'""",
                       (text, json.dumps(tools), json.dumps(transcript), status, error, tid))
            if status != "generating":
                db.execute("""UPDATE conversations SET updated_at=? WHERE id=
                              (SELECT conversation_id FROM turns WHERE id=?)""", (now(), tid))
