"""HTTP acceptance test against the compiled server and real Poppler tools."""

import json
import os
import re
import socket
import sqlite3
import subprocess
import tempfile
import time
import unittest
import urllib.error
import urllib.parse
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def pdf():
    text = b"BT /F1 12 Tf 40 750 Td (E-Permit) Tj 0 -22 Td (Numero de permis : HTTP-1) Tj 0 -22 Td (Etat : Valide) Tj 0 -22 Td (Date de fin : 31/12/2027) Tj ET"
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
               b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
               b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
               b"<< /Length %d >>\nstream\n" % len(text) + text + b"\nendstream"]
    content = b"%PDF-1.4\n"
    offsets = []
    for index, obj in enumerate(objects, 1):
        offsets.append(len(content))
        content += b"%d 0 obj\n" % index + obj + b"\nendobj\n"
    xref = len(content)
    content += b"xref\n0 6\n0000000000 65535 f \n"
    for offset in offsets:
        content += b"%010d 00000 n \n" % offset
    return content + b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF" % xref


class Fields(HTMLParser):
    def __init__(self, html):
        super().__init__()
        self.values = {}
        self.textarea = None
        self.feed(html)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "input" and "name" in attrs:
            self.values[attrs["name"]] = attrs.get("value", "")
        if tag == "textarea":
            self.textarea = attrs.get("name")
            self.values[self.textarea] = ""

    def handle_data(self, text):
        if self.textarea:
            self.values[self.textarea] += text

    def handle_endtag(self, tag):
        if tag == "textarea":
            self.textarea = None


class HttpAcceptance(unittest.TestCase):
    def test_real_upload_review_validation_and_restart(self):
        binary = Path(os.environ.get("PERMIT_ALERT_TEST_BINARY", ROOT / "app/target/debug/permit-alert")).resolve()
        with tempfile.TemporaryDirectory() as directory:
            with socket.socket() as available:
                available.bind(("127.0.0.1", 0))
                port = available.getsockname()[1]
            base = f"http://127.0.0.1:{port}"
            database = Path(directory) / "state.sqlite3"
            env = dict(os.environ, PERMIT_ALERT_DATABASE=str(database), PERMIT_ALERT_LISTEN=f"127.0.0.1:{port}", PERMIT_ALERT_PUBLIC_URL=base, PERMIT_ALERT_BASE_PATH="", PERMIT_ALERT_TEMP=directory)

            def start():
                process = subprocess.Popen([str(binary)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
                for _ in range(100):
                    if process.poll() is not None:
                        raise RuntimeError(process.stderr.read().decode())
                    try:
                        urllib.request.urlopen(base + "/health", timeout=1).close()
                        return process
                    except (OSError, urllib.error.URLError):
                        time.sleep(.05)
                process.kill()
                raise RuntimeError("Server did not start")

            process = start()
            try:
                html = urllib.request.urlopen(base + "/imports").read().decode()
                csrf = Fields(html).values["csrf"]
                boundary = "permit-alert-test-boundary"
                body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"csrf\"\r\n\r\n{csrf}\r\n"
                        f"--{boundary}\r\nContent-Disposition: form-data; name=\"pdf\"; filename=\"permit.pdf\"\r\nContent-Type: application/pdf\r\n\r\n").encode() + pdf() + f"\r\n--{boundary}--\r\n".encode()
                response = urllib.request.urlopen(urllib.request.Request(base + "/imports", data=body, headers={"Content-Type": f"multipart/form-data; boundary={boundary}"}))
                review_url = response.url
                fields = Fields(response.read().decode()).values
                self.assertEqual(fields["0.number"], "HTTP-1")
                with sqlite3.connect(database) as db:
                    self.assertEqual(db.execute("SELECT COUNT(*) FROM permits").fetchone(), (0,))
                fields["0.unit"] = "Unité corrigée"
                fields["0.extra"] = json.dumps({"Zone": "Z42"})
                fields["confirmed"] = "yes"
                saved = urllib.request.urlopen(review_url + "/validate", data=urllib.parse.urlencode(fields).encode()).read().decode()
                self.assertIn("HTTP-1", saved)
                self.assertIn("Unité corrigée", saved)
                self.assertEqual(list(Path(directory).rglob("*.pdf")), [])
                with sqlite3.connect(database) as db:
                    self.assertEqual(db.execute("SELECT COUNT(*) FROM drafts").fetchone(), (0,))
                    data = json.loads(db.execute("SELECT data FROM permits").fetchone()[0])
                    self.assertEqual(data["extra"]["Zone"], "Z42")
                try:
                    urllib.request.urlopen(base + "/settings", data=b"lead_days=3&frequency_days=1")
                    self.fail("CSRF should be denied")
                except urllib.error.HTTPError as error:
                    self.assertEqual(error.code, 403)
                    error.close()
                pending = urllib.request.urlopen(urllib.request.Request(base + "/imports", data=body, headers={"Content-Type": f"multipart/form-data; boundary={boundary}"}))
                pending_url = pending.url
                pending.close()
                self.assertEqual(len(list(Path(directory).rglob("*.pdf"))), 1)
                process.terminate()
                process.wait(timeout=5)
                process.stderr.close()
                process = start()
                self.assertIn("HTTP-1", urllib.request.urlopen(base + "/").read().decode())
                resumed = urllib.request.urlopen(pending_url).read().decode()
                self.assertIn("Fiche existante", resumed)
                self.assertTrue(urllib.request.urlopen(pending_url + "/pdf").read().startswith(b"%PDF-"))
                cancel = urllib.parse.urlencode({"csrf": Fields(resumed).values["csrf"]}).encode()
                urllib.request.urlopen(pending_url + "/cancel", data=cancel).close()
                self.assertEqual(list(Path(directory).rglob("*.pdf")), [])
            finally:
                process.terminate()
                process.wait(timeout=5)
                process.stderr.close()


if __name__ == "__main__":
    unittest.main()
