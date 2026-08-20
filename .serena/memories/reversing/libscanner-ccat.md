# libscanner.so / CCat reverse engineering (FW 5.19.2, rootfs at /home/marek/Downloads/5192/rootfs)

## Extractor contract (verified in BN, libscanner.so.1.0)
- Event struct passed to extractor callback, 24 bytes (memset 0x18 in `scanner_send_event_type` @0x18fac):
  - +0x0 event_type (int32), +0x4 path (dir), +0x8 lipc handle, +0xc filename, +0x10 uuid, +0x14 glob (set by scanner_handle @0x181b4 before call: `*(event+0x14)=matched_glob`)
  - Confirmed by `make_path_from_scanner_event` in libextractor_util @0x12438: `concat_path_file(*(e+4),*(e+0xc))`
- Event types: 0=ADD,1=DELETE,2=UPDATE,3=ADD_THUMB,4=UPDATE_THUMB (strings in scanner_send_event_type)
- `scanner_handle` matches extractor glob with `fnmatch(glob, filename, 0x10)` (0x10=FNM_CASEFOLD), then calls registered handler `(entry+4)(event, entry+0xc)`; entry+0xc is unk1 (=0).
- Extractor loaded via `load_handler_lib` @0x18938: dlopen(so), `dlsym(h, ep)` where ep + so path + glob come from the AppReg registry (libappreg), called as `ep(&entry->handler, &entry->unk1)` — signature matches our `load_extractor`.
- HEADER: extractor returns 0 = processed; scanner counter data_2b4c4++ when >0 (log "extractor problem").

## HTTP endpoints (what scanner_post_* do)
- `scanner_post_string` @0x15e8c: POST to `http://localhost:9101/change`, retries 3x with usleep(1s), expects response JSON {"ok":true,...}; ccat replies `200 OK\n{"ok":true,"changes":N,"type":"ChangeResponse"}`.
- `scanner_post_to_uri_internal` @0x14c70: curl POST, header `AuthToken: <file /tmp/session_token>` (get_lab126_session_token = read /tmp/session_token), retries HTTP 503 up to 3x backoff 1000/2000/4000us, and **adds `"contentSource":"OnDevice"`** top-level to body (MANDATORY for sideload, else ccat treats as cloud update path).
- `scanner_delete_ccat_entry` @0x1631c JSON: {"type":"ChangeRequest","commands":[{"delete":{"uuid":U},"updateDeletedArchivedItem":{"deletedUuid":U}}],"caller":"scannerDelete"}
- `scanner_gen_uuid` @0x14984 = libuuid uuid_generate+uuid_unparse_lower. `getSha1Hash` @0x1de48 = SHA1 hex.

## CCat server = Lua, /usr/lib/ccat/*.lua (worker.lua maps process_/query -> query.query, process_/change -> change.change)
- /change accepts: type="ChangeRequest", commands=[{insert|update|delete|insertOr|resetIndexer|...}], caller, contentSource, noNotify (bool: skip UI refresh notification).
- insert entry fields (change.lua column_specs ~line 755): uuid,type,location,mimeType,cdeKey,cdeType,contentSize,diskUsage,modificationTime,publicationDate,publisher,languages,credits,titles,displayObjects,displayTags,thumbnail,isArchived,isVisibleInHome,percentFinished,contentIndexedState,noteIndexedState,lastAccess,guid,version,cover,ownershipType,originType,etc. Unknown field => assert error "Attempt to set unknown field".
- insertOr: {entry:{...}, filter:{Equals:{path:"location",value:...}}, onConflict:"REPLACE"} — stock scanner uses this (build_minimal_insertOr @0x14a08).
- Titles/credits defaulted server-side if missing; ccat auto-indexes title/content for search via location + mimeType.
- cdeType values in ccat: EBOK, EBSP, AUDI, AUSP, PDOC, MAGZ, NWPR, FEED. PDOC special: publisher fallback (fallbackPDocPublisher), purchaseDate fallback. EBOK triggers audible-companion logic.
- `is_pdoc(mimeType, cdeType)` in libextractor_util @0x144c4: true if mimeType NULL or starts with '*' or cdeType=="PDOC". `isNewTag` @0x157a8: reads cJSON bool "newTag". Legacy metadata builder `create_JSON_metadata_entry` @0x11ee0 shows canonical entry shape (titles/credits/displayObjects refs titles+credits/displayTags/cdeKey/cdeType/contentIndexedState/contentSize).
- The stock extractor for sideloaded KFX etc. is in /app/lib/libKindleEinkYJExtractorShared.so (uses is_pdoc, isNewTag, displayTags, sendNewItemAddedEvent).

## IMPLEMENTED (kompanion, Aug 2026): self-posted CCat HTTP + per-format cdeType
- cjson-bindings REMOVED everywhere. generate_change_request (kompanion_core) now returns
  serde_json::Value; contentSource "OnDevice" + "caller" stamped at POST time.
- kompanion_sys/src/ccat.rs: raw-TcpStream HTTP/1.1 POST to 127.0.0.1:9101/change (no new deps);
  AuthToken header from /tmp/session_token; retries 4x (1s sleep, 503→ 1000/2000/4000us backoff);
  parses `{"ok":true}` from plain "200 OK\n{...}" or full HTTP responses. delete = delete+updateDeletedArchivedItem pair (caller scannerDelete). update_thumbnail = update command.
- RealScanner (feature real-scanner): post_change/delete/update go over HTTP; only scanner_gen_uuid
  still links libscanner. get_thumbnail_for_uuid returns None (unused). get_sha1_hash removed from trait.
- cde_type() added to FileIndexer trait (default PDOC); EPUB + FB2 return EBOK (→ rawBookType EBOK
  = EBooks search filter, book classification); CBZ stays PDOC (no comics cdeType).
- Extra fields now sent on insert: languages (extra["language"]→ array, feeds titles[1].language),
  publisher, publicationDate (validated ccat columns; others like description/series/subjects skipped
  => NOT ccat entry columns, would assert).
- pack.sh: dropped cjson_addboolfixup compile + RUSTFLAGS for extractor; keep getauxval stub for pw2.
  NOTE: this host has no arm-kindlepw2 gcc (binutils only) so pw2 can't link here (pre-existing).
  hf build verified OK; only remaining undefined import in the .so is scanner_gen_uuid.

## Tag/filter systems on FW 5.19.2 (KPPMainAppV2 search + ignite library)
- Chrome/device search = "oob-search" providers (KPPChromeSearchModule); embedded search engine
  `docsearch` lives in KPPMainAppV2 (stripped, 40MB, ARM32). libccat exposes si_bridge_* DATA SLOTS
  (OBJECT reloc, section 23; e.g. si_bridge_index_title @0x106d0) filled by the app at runtime; Lua
  index_title() wraps that slot.
- index_title(asin, cdeType, guid, location, uuid, content_language, mimeType) called from change.lua:718
  on every insert. NO tags argument. (delete_index(location) from dcm.lua:348.)
- Library search filter DSL (embedded filterProviderList JSON, app string @34971965):
  * EBooks:    rawBookType == 'EBOK' OR 'YJOP'
  * Samples:   rawBookType == 'EBSP'
  * Documents: origin CONTAINS 'PDocs'
  * Newsstand: rawBookType == 'NWPR' OR 'MAGZ'
  * Audible:   rawBookType == 'Audible'
  * Comics:    contentTags CONTAINS 'MANGA' OR contentTags CONTAINS 'COMICS'
- rawBookType maps from cdeType. contentTags appears ONLY in the filter JSON (no other literal in the
  app) -> server/Comixology-fed (SMD fields: meta_data, cde_contenttype, content_size, origins);
  sideloads CANNOT get COMICS via ccat /change. No comic cdeType exists in ccat (only
  EBOK/EBSP/AUDI/AUSP/PDOC/MAGZ/NWPR/FEED); comics share EBOK.
- ignite asset_db migration SQL (app string @36010303): Nodes ASSET_ID = ASIN||"!!"||CASE TYPE:
  BOOK->EBOK, SAMPLE->EBSP, AUDIOBOOK->AUDI, AUDIOBOOK_SAMPLE->AUSP, MAGAZINE->MAGZ, NEWSPAPER->NWPR,
  DOCUMENT->PDOC, DICTIONARY->EBOK, COMIC->EBOK, PERSONAL_LETTER->PSNL, FEED->FEED, SERIES->series.
- ignite asset classes: ksdk::ignite::{Book,Comic,Magazine,Newspaper,PersonalDocument,Dictionary,
  Sample,Tag,Genre,Series,Folder,Notebook,Audible,QuickNotes}Asset. Sideload path -> PersonalDocumentAsset
  for PDOC; NO path to ComicAsset via our cdeType.
- Layering of "tags": displayTags (ccat entry -> NEW badge; isNewTag) != contentTags (search
  classification, server-fed) != ContentTag/Tags tables (ignite asset_db: user Collections/Favorites;
  kFavoritesSystemTagCreation, TagAsset, TagRelationBackingModel) != LibraryTag DTO (library/dal/eink:
  kNewTag, kKeepTag + TagUtils::GetDisplayTags(vector<string>,clear,set<LibraryTag>)).
- Nodes columns: INTERNAL_ID, ASSET_ID, ASIN, TYPE, TITLE, TITLE_COLLATION, PARENT_*, GENRE_RANK,
  SYNC_STATE, AUDIBLE_*, TOTAL_POSITION, ADDITIONAL_DATA, RECAP_ENABLED, ORIGIN, DOWNLOAD_STATE,
  ARCHIVE, LOCATION, MIMETYPE, SIDELOAD, AUTHORS, AUTHORS_COLLATION, PUBLISHER, PUBLICATION_DATE,
  THUMBNAIL, MODIFICATION_DATE, GUID, CREATION_DATE, ENCRYPT, LAST_ACCESS_TIME, PARENT_FOLDER_*,
  SYSTEM_HIDDEN, PARENT_HASH. No contentTags/tags column.