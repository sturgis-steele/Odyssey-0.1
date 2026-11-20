# Product roadmap

| Stage | Product work | Entry decision |
| --- | --- | --- |
| 1 · Local assistant | Touch console, conversations, local model adapter and three read-only tools | Validate the selected model/template and the target Pi |
| 2 · Offline knowledge | Independent collection browser and retrieval services for curated ZIMs, metadata and citations | Choose one licensed reference collection, its format and an acceptance query set |
| 3 · Personal assistant | Portable Markdown notes, calendar, personal preferences and durable memory | Define permissions, identity boundaries, retention, migration and backup policies |
| 4 · Field operation | Offline maps, voice input/output and resilience workflows | Measure storage, audio hardware, GPS requirements and power budgets |
| 5 · Commercial device | Hardware integration, reproducible offline image, updates, recovery, manufacturing and support | Complete hardware qualification and redistribution review |

## Decisions before Stage 2

Choose a small ZIM collection and test lookup and browsing independently of inference. Place collection operations behind a read-only service. An approved search tool can then return source identifiers and excerpts through the existing tool interface. Keep personal SQLite data, read-only collections and future embedding indexes separately replaceable.

Measure the target hardware with a transcript set containing follow-up questions, refusals of unavailable capabilities, malformed tool arguments and model loss. Select the shipping model only after reviewing those results and the exact file's license.

## Technical debt and distribution

- The device handles one generation at a time. It has no multi-user authentication, streaming resume protocol, personal memory or semantic history summarization. Plaintext SQLite relies on the local OS account for access control.
- Checkpointing can lose the most recent uncommitted text at sudden power loss. Schema version 1 has an explicit forward migration boundary. Test migrations with real backups before introducing a second schema.
- Review licenses and notices for runtime dependencies, native CPU builds, fonts, icons, weights, templates and each future dataset. Decide Odyssey's own distribution terms before publication.
- Offline installation media, per-platform hashes, signed updates, real-device benchmarks, backup UX and read/write permission prompts for future tools remain productization work.
