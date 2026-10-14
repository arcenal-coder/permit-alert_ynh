import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("snapshot", ROOT / "scripts/backup-sqlite.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PackageRegressionTests(unittest.TestCase):
    def test_manifest_has_metadata_required_by_webadmin(self):
        manifest = tomllib.loads((ROOT / "manifest.toml").read_text())
        # AppInstall.vue dereferences upstream.demo and upstream.license.
        # The absence of the upstream object leaves the form in loading state.
        self.assertIsInstance(manifest["upstream"], dict)
        self.assertIn("code", manifest["upstream"])
        self.assertIn(
            manifest["install"]["init_main_permission"]["default"],
            ("visitors", "all_users", "admins"),
        )

    def test_snapshot_restores_committed_data_from_live_wal_database(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "live.sqlite3"
            destination = Path(directory) / "backup.sqlite3"
            with sqlite3.connect(source) as live:
                live.execute("PRAGMA journal_mode=WAL")
                live.execute("CREATE TABLE settings (key TEXT, value TEXT)")
                live.execute("INSERT INTO settings VALUES ('delay', '7')")
                live.commit()
                module.snapshot(source, destination)
                with sqlite3.connect(destination) as restored:
                    self.assertEqual(
                        restored.execute("SELECT * FROM settings").fetchall(),
                        [("delay", "7")],
                    )


if __name__ == "__main__":
    unittest.main()
