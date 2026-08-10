BEGIN;
DELETE FROM mimetypes WHERE ext='epub';
DELETE FROM extenstions WHERE ext='epub';

INSERT INTO mimetypes (ext, mimetype) VALUES ('epub', 'MT:application/epub+zip');
INSERT INTO extenstions (ext, mimetype) VALUES ('epub', 'MT:application/epub+zip');

-- Launcher: clean old rows first (safe reinstall)
DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.launcher';
DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.launcher';
DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.launcher';

INSERT INTO handlerIds (handlerId) VALUES ('com.notmarek.kompanion.launcher');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'extend-start', 'Y');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'unloadPolicy', 'unloadOnPause');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'maxGoTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'maxPauseTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'maxUnloadTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'maxLoadTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.launcher', 'command', '/var/local/kompanion/bin/kompanion_launcher');
INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('application', 'com.notmarek.kompanion.launcher', 'MT:application/epub+zip', 'true');

-- Extractor: clean old rows first (safe reinstall), path set by install.sh
DELETE FROM associations WHERE handlerId = 'com.notmarek.kompanion.extractor';
DELETE FROM properties WHERE handlerId = 'com.notmarek.kompanion.extractor';
DELETE FROM handlerIds WHERE handlerId = 'com.notmarek.kompanion.extractor';

INSERT INTO handlerIds (handlerId) VALUES ('com.notmarek.kompanion.extractor');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.extractor', 'lib', '@EXTRACTOR_LIB@');
INSERT INTO properties (handlerId, name, value) VALUES ('com.notmarek.kompanion.extractor', 'entry', 'load_extractor');
INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('extractor', 'com.notmarek.kompanion.extractor', 'GL:*.epub', 'true');
COMMIT;
