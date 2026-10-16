"""Produce a consistent SQLite snapshot without copying temporary files."""

import sqlite3
import sys
from pathlib import Path


def snapshot(source, destination):
    with sqlite3.connect(Path(source).resolve().as_uri() + "?mode=ro", uri=True) as live:
        with sqlite3.connect(destination) as backup:
            live.backup(backup)
            # Validation drafts refer to temporary PDFs, which must never be
            # restored or backed up. Keep only approved business data.
            if backup.execute("SELECT 1 FROM sqlite_master WHERE name='drafts'").fetchone():
                backup.execute("DELETE FROM drafts")
            if backup.execute("PRAGMA integrity_check").fetchone() != ("ok",):
                raise RuntimeError("SQLite backup integrity check failed")


if __name__ == "__main__":
    snapshot(sys.argv[1], sys.argv[2])
