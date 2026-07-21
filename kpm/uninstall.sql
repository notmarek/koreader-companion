BEGIN;
DELETE FROM mimetypes WHERE ext='epub';
DELETE FROM extenstions WHERE ext='epub';

-- Remove associations and properties for the launcher
DELETE FROM associations WHERE handlerId = 'github.koreader.companion.launcher';
DELETE FROM properties WHERE handlerId = 'github.koreader.companion.launcher';
DELETE FROM handlerIds WHERE handlerId = 'github.koreader.companion.launcher';

-- Remove associations and properties for the extractor
DELETE FROM associations WHERE handlerId = 'github.koreader.companion.extractor';
DELETE FROM properties WHERE handlerId = 'github.koreader.companion.extractor';
DELETE FROM handlerIds WHERE handlerId = 'github.koreader.companion.extractor';
COMMIT;
