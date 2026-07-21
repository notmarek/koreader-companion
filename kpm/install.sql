BEGIN;
DELETE FROM mimetypes WHERE ext='epub';
DELETE FROM extenstions WHERE ext='epub';

INSERT INTO mimetypes (ext, mimetype) VALUES ('epub', 'MT:application/epub+zip');
INSERT INTO extenstions (ext, mimetype) VALUES ('epub', 'MT:application/epub+zip');

INSERT INTO handlerIds (handlerId) VALUES ('github.koreader.companion.launcher');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'extend-start', 'Y');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'unloadPolicy', 'unloadOnPause');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'maxGoTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'maxPauseTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'maxUnloadTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'maxLoadTime', '60');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.launcher', 'command', '/var/local/kmc/bin/koreader_companion_launcher');
INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('application', 'github.koreader.companion.launcher', 'application/epub+zip', 'true');


INSERT INTO handlerIds (handlerId) VALUES ('github.koreader.companion.extractor');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.extractor', 'lib', '/var/local/kmc/lib/libkoreader_companion_extractor.so');
INSERT INTO properties (handlerId, name, value) VALUES ('github.koreader.companion.extractor', 'entry', 'load_extractor');
INSERT INTO associations (interface, handlerId, contentId, defaultAssoc) VALUES ('extractor', 'github.koreader.companion.extractor', 'GL:*.epub', 'true');
COMMIT;
