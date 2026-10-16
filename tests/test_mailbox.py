import base64
import importlib.util
import unittest
import subprocess
import sys
from types import SimpleNamespace
from email.message import EmailMessage
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("mailbox_worker", ROOT / "scripts/mailbox.py")
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


def message():
    mail = EmailMessage()
    mail["From"] = "coordinateur@example.org"
    mail["To"] = "epermit@example.org"
    mail["Authentication-Results"] = "forged.example; dkim=pass; dmarc=pass"
    mail.set_content("Import")
    mail.add_attachment(b"%PDF-1.7\nexample", maintype="application", subtype="pdf", filename="permit.pdf")
    return mail


class MailboxTests(unittest.TestCase):
    @patch.object(worker, "aligned_dkim", return_value=True)
    @patch.object(worker, "authorized_sender", return_value=False)
    def test_outside_group_rejected_before_authentication(self, authorize, verify):
        result = worker.inspect(message().as_bytes())
        self.assertIn("non membre", result["reason"])
        self.assertNotIn("pdf", result)
        verify.assert_not_called()

    @patch.object(worker, "aligned_dkim", return_value=False)
    @patch.object(worker, "authorized_sender", return_value=True)
    def test_forged_authentication_results_not_trusted(self, authorize, verify):
        result = worker.inspect(message().as_bytes())
        self.assertIn("DKIM", result["reason"])
        self.assertNotIn("pdf", result)

    @patch.object(worker, "aligned_dkim", return_value=True)
    @patch.object(worker, "authorized_sender", return_value=True)
    def test_authorized_single_pdf_and_ambiguous_attachments(self, authorize, verify):
        mail = message()
        result = worker.inspect(mail.as_bytes())
        self.assertTrue(base64.b64decode(result["pdf"]).startswith(b"%PDF-"))
        mail.add_attachment(b"%PDF-1.7", maintype="application", subtype="pdf", filename="other.pdf")
        result = worker.inspect(mail.as_bytes())
        self.assertIn("Un seul PDF", result["reason"])

    def test_ldif_and_ldap_injection(self):
        self.assertEqual(worker.ldif_values("memberUid: alice\nmemberUid:: Ym9i\n", "memberUid"), ["alice", "bob"])
        self.assertEqual(worker.ldap_escape("a*)(uid=*)"), r"a\2a\29\28uid=\2a\29")

    @patch.object(worker, "ldap_search", return_value="")
    def test_missing_group_is_a_configuration_failure_not_a_message_rejection(self, search):
        with self.assertRaises(RuntimeError):
            worker.authorized_sender("coordinateur@example.org", {"group": "absent", "groups_base": "ou=groups,dc=example", "users_base": "ou=users,dc=example"})


@unittest.skipUnless(importlib.util.find_spec("dkim"), "requires dkimpy and dnspython")
class CryptographicAuthenticationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.private = subprocess.run(["openssl", "genrsa", "2048"], capture_output=True, check=True).stdout
        public = subprocess.run(["openssl", "rsa", "-pubout"], input=cls.private, capture_output=True, check=True).stdout
        cls.dns = b"v=DKIM1; k=rsa; p=" + b"".join(public.splitlines()[1:-1])

    def signed(self, domain=b"example.org", partial=False):
        import dkim
        raw = message().as_bytes(policy=email_policy())
        return dkim.sign(raw, b"test", domain, self.private,
                         include_headers=[b"from", b"to", b"mime-version", b"content-type"],
                         length=partial) + raw

    @patch.object(worker, "authorized_sender", return_value=True)
    def test_valid_signature_tampering_alignment_and_partial_signature(self, authorized):
        from email.parser import BytesParser
        with patch("dns.resolver.resolve", return_value=[SimpleNamespace(strings=[self.dns])]):
            valid = self.signed()
            self.assertIn("pdf", worker.inspect(valid))
            tampered = valid.replace(b"Import", b"Forged")
            self.assertIn("DKIM", worker.inspect(tampered)["reason"])
            mismatched = self.signed(b"attacker.org")
            self.assertIn("DKIM", worker.inspect(mismatched)["reason"])
            partial = self.signed(partial=True)
            self.assertIn("DKIM", worker.inspect(partial)["reason"])


def email_policy():
    from email.policy import SMTP
    return SMTP


if __name__ == "__main__":
    unittest.main()
