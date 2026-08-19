BEGIN;
DELETE FROM mimetypes WHERE ext IN ('epub', 'cbz');
DELETE FROM extenstions WHERE ext IN ('epub', 'cbz');

-- Remove associations and properties for the launcher
DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.launcher';
DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.launcher';
DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.launcher';

-- Remove associations and properties for the extractor
DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.extractor';
DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.extractor';
DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.extractor';
COMMIT;
