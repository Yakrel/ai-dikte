#!/bin/sh
# Called by package builds from repository root. No live-system writes.
set -eu
: "${DESTDIR:?Set DESTDIR to the package staging directory}"
: "${AI_DIKTE_BINARY:?Set AI_DIKTE_BINARY to the compiled Rust executable}"
install -Dm755 "$AI_DIKTE_BINARY" "$DESTDIR/usr/bin/ai-dikte"
install -Dm755 ai-dikte-toggle "$DESTDIR/usr/bin/ai-dikte-toggle"
install -Dm644 ai-dikte.png "$DESTDIR/usr/share/icons/hicolor/256x256/apps/ai-dikte.png"
install -Dm644 ai-dikte.desktop "$DESTDIR/usr/share/applications/ai-dikte.desktop"
install -Dm644 ai-dikte.desktop "$DESTDIR/usr/share/kglobalaccel/ai-dikte.desktop"
install -Dm644 ai-dikte-settings.desktop "$DESTDIR/usr/share/applications/ai-dikte-settings.desktop"
install -Dm644 packaging/linux/ai-dikte.service "$DESTDIR/usr/lib/systemd/user/ai-dikte.service"
install -Dm644 LICENSE "$DESTDIR/usr/share/licenses/ai-dikte/LICENSE"
