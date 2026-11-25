use crate::error::{Error, Result};
use axum::http::StatusCode;
use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub const INTERRUPTED: &str = "Generation was interrupted. Send another message to continue.";
type Job = Box<dyn FnOnce(&mut Connection) + Send>;
#[derive(Clone)]
pub struct Store {
    queue: mpsc::Sender<Job>,
}

fn stamp() -> String {
    format!("{:.6}", Timestamp::now()).replace('Z', "+00:00")
}
fn summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,
              "created_at":row.get::<_,String>(2)?,"updated_at":row.get::<_,String>(3)?}),
    )
}
fn get(db: &Connection, cid: &str) -> Result<Value> {
    db.query_row(
        "SELECT id,title,created_at,updated_at FROM conversations WHERE id=?",
        [cid],
        summary,
    )
    .optional()?
    .ok_or_else(|| {
        Error::new(
            "not_found",
            "Conversation could not be found.",
            StatusCode::NOT_FOUND,
        )
    })
}
fn initialize(path: &std::path::Path) -> std::result::Result<(Connection, Connection), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Cannot create database directory")?;
    }
    let mut lock_name = path.as_os_str().to_owned();
    lock_name.push(".lock");
    let lock = Connection::open(PathBuf::from(lock_name))
        .map_err(|_| "Cannot open database ownership lock")?;
    lock.busy_timeout(Duration::ZERO)
        .map_err(|_| "Cannot configure ownership lock")?;
    lock.execute_batch("BEGIN EXCLUSIVE")
        .map_err(|_| "Database is owned by another process or unavailable")?;
    let mut db = Connection::open(path).map_err(|_| "Cannot open local database")?;
    let init = || "Cannot initialize SQLite database";
    db.busy_timeout(Duration::from_secs(1))
        .map_err(|_| init())?;
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| init())?;
    if version > 1 {
        return Err("The database requires a newer version of Odyssey.".into());
    }
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
        .map_err(|_| init())?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| init())?;
    if version == 0 {
        tx.execute_batch(include_str!("../migrations/001.sql"))
            .map_err(|_| init())?;
        tx.pragma_update(None, "user_version", 1)
            .map_err(|_| init())?;
    }
    tx.execute(
        "UPDATE turns SET status='interrupted',error=? WHERE status='generating'",
        [INTERRUPTED],
    )
    .map_err(|_| init())?;
    tx.commit().map_err(|_| init())?;
    Ok((db, lock))
}
impl Store {
    pub async fn open(path: PathBuf) -> std::result::Result<Self, String> {
        let (queue, mut jobs) = mpsc::channel::<Job>(32);
        let (ready, wait) = oneshot::channel();
        std::thread::Builder::new()
            .name("odyssey-sqlite".into())
            .spawn(move || match initialize(&path) {
                Ok((mut db, lock)) => {
                    if ready.send(Ok(())).is_ok() {
                        while let Some(job) = jobs.blocking_recv() {
                            job(&mut db);
                        }
                    }
                    drop(db);
                    drop(lock);
                }
                Err(e) => {
                    let _ = ready.send(Err(e));
                }
            })
            .map_err(|_| "Cannot start SQLite worker")?;
        wait.await.map_err(|_| "SQLite worker stopped")??;
        Ok(Self { queue })
    }
    async fn run<T: Send + 'static>(
        &self,
        op: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (send, recv) = oneshot::channel();
        self.queue
            .send(Box::new(move |db| {
                let _ = send.send(op(db));
            }))
            .await
            .map_err(|_| Error::storage())?;
        recv.await.map_err(|_| Error::storage())?
    }
    pub async fn ping(&self) -> Result<()> {
        self.run(|db| {
            db.query_row("SELECT EXISTS(SELECT 1 FROM conversations)", [], |_| Ok(()))?;
            Ok(())
        })
        .await
    }
    pub async fn create(&self) -> Result<Value> {
        self.run(|db|{
            let cid=Uuid::new_v4().to_string();
            let time=stamp();
            let tx=db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute("INSERT INTO conversations (id,title,created_at,updated_at) VALUES (?,'New conversation',?,?)",params![cid,time,time])?;
            let row=get(&tx,&cid)?;
            tx.commit()?;
            Ok(row)
        }).await
    }
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Value>> {
        self.run(move|db|{
            let mut stmt=db.prepare("SELECT id,title,created_at,updated_at FROM conversations ORDER BY updated_at DESC,id LIMIT ? OFFSET ?")?;
            Ok(stmt.query_map((limit,offset),summary)?.collect::<rusqlite::Result<_>>()?)
        }).await
    }
    pub async fn detail(&self, cid: String, limit: i64, before: i64) -> Result<Value> {
        self.run(move|db|{
            let mut out=get(db,&cid)?;
            let mut stmt=db.prepare("SELECT seq,id,created_at,user,assistant,status,error,activity FROM turns WHERE conversation_id=? AND seq<? ORDER BY seq DESC LIMIT ?")?;
            let mut rows=stmt.query(params![cid,before,limit+1])?;
            let mut turns=Vec::new();
            while let Some(r)=rows.next()? {
                let activity:String=r.get(7)?;
                let tools:Vec<Value>=serde_json::from_str(&activity).map_err(|_|Error::storage())?;
                turns.push(json!({"seq":r.get::<_,i64>(0)?,"id":r.get::<_,String>(1)?,"created_at":r.get::<_,String>(2)?,
                    "user":r.get::<_,String>(3)?,"assistant":r.get::<_,String>(4)?,"status":r.get::<_,String>(5)?,
                    "error":r.get::<_,Option<String>>(6)?,"tools":tools}));
            }
            let more=turns.len()>limit as usize;
            turns.truncate(limit as usize);
            turns.reverse();
            out["next_before"]=if more {turns[0]["seq"].clone()} else {Value::Null};
            out["turns"]=Value::Array(turns);
            Ok(out)
        }).await
    }
    pub async fn delete(&self, cid: String) -> Result<()> {
        self.run(move|db|{
            let tx=db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            get(&tx,&cid)?;
            let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM turns WHERE conversation_id=? AND status='generating')",[&cid],|r|r.get(0))?;
            if busy {return Err(Error::new("busy","Stop the active response before deleting this conversation.",StatusCode::CONFLICT));}
            tx.execute("DELETE FROM conversations WHERE id=?",[cid])?;
            tx.commit()?;
            Ok(())
        }).await
    }
    pub async fn begin(&self, cid: String, user: String) -> Result<String> {
        self.run(move|db|{
            let tx=db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            get(&tx,&cid)?;
            if tx.query_row("SELECT EXISTS(SELECT 1 FROM turns WHERE status='generating')",[],|r|r.get::<_,bool>(0))? {return Err(Error::busy());}
            let tid=Uuid::new_v4().to_string();
            let time=stamp();
            let first:bool=tx.query_row("SELECT NOT EXISTS(SELECT 1 FROM turns WHERE conversation_id=?)",[&cid],|r|r.get(0))?;
            tx.execute("INSERT INTO turns(id,conversation_id,created_at,user,status) VALUES(?,?,?,?,'generating')",params![tid,cid,time,user])?;
            if first {
                let title:String=user.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(68).collect();
                tx.execute("UPDATE conversations SET title=?,updated_at=? WHERE id=?",params![title,time,cid])?;
            } else {tx.execute("UPDATE conversations SET updated_at=? WHERE id=?",params![time,cid])?;}
            tx.commit()?;
            Ok(tid)
        }).await
    }
    pub async fn history(&self, cid: String, limit: usize) -> Result<Vec<Vec<Value>>> {
        self.run(move|db|{
            let mut stmt=db.prepare("SELECT transcript FROM turns WHERE conversation_id=? AND status='complete' ORDER BY seq DESC LIMIT ?")?;
            let mut rows=stmt.query(params![cid,limit as i64])?;
            let mut history=Vec::new();
            while let Some(row)=rows.next()? {
                let raw:String=row.get(0)?;
                history.push(serde_json::from_str(&raw).map_err(|_|Error::storage())?);
            }
            history.reverse();
            Ok(history)
        }).await
    }
    pub async fn save(
        &self,
        tid: String,
        text: String,
        activity: Vec<Value>,
        mut transcript: Vec<Value>,
        status: &'static str,
        error: Option<&'static str>,
    ) -> Result<()> {
        self.run(move|db|{
            for message in &mut transcript {
                if let Some(obj)=message.as_object_mut(){obj.remove("reasoning_content");}
            }
            let tx=db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let changed=tx.execute("UPDATE turns SET assistant=?,activity=?,transcript=?,status=?,error=? WHERE id=? AND status='generating'",
                params![text,Value::Array(activity).to_string(),Value::Array(transcript).to_string(),status,error,tid])?;
            if changed>0 && status!="generating" {
                tx.execute("UPDATE conversations SET updated_at=? WHERE id=(SELECT conversation_id FROM turns WHERE id=?)",params![stamp(),tid])?;
            }
            tx.commit()?;
            Ok(())
        }).await
    }
}
