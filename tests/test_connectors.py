import importlib.util
import json
import os
from pathlib import Path
import ssl
import smtplib
import subprocess
import sys
import unittest
from unittest.mock import MagicMock, patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import connector_io
import connectors


def configuration():
    return {"outgoing": {"host": "smtp.example.org", "port": 587, "tls": "starttls", "username": "sender", "password": "PRIVATE_CONNECTOR_TEST_VALUE", "sender": "sender@example.org"},
            "imap": {"host": "imap.example.org", "port": 993, "username": "inbox", "password": "PRIVATE_CONNECTOR_TEST_VALUE", "folder": "INBOX"},
            "ldap": {"url": "ldap://127.0.0.1", "users_base": "ou=users,dc=example", "groups_base": "ou=groups,dc=example", "group": "coordinateurs", "bind_dn": "", "bind_password": ""}}


class ConnectorTests(unittest.TestCase):
    @patch("connector_io.imaplib.IMAP4_SSL")
    def test_imap_diagnostic_is_read_only_and_does_not_fetch_or_delete(self, factory):
        client = factory.return_value.__enter__.return_value
        client.capabilities = (b"IMAP4REV1", b"UIDPLUS")
        client.select.return_value = ("OK", [b"10"])
        client.noop.return_value = ("OK", [b"ready"])
        result = connectors.execute("test-imap", {"config": configuration()})
        self.assertTrue(result["ok"])
        client.select.assert_called_once_with("INBOX", readonly=True)
        client.uid.assert_not_called()
        client.fetch.assert_not_called()
        client.store.assert_not_called()
        client.expunge.assert_not_called()
        context = factory.call_args.kwargs["ssl_context"]
        self.assertTrue(context.check_hostname)
        self.assertEqual(context.verify_mode, ssl.CERT_REQUIRED)

    @patch("connector_io.smtplib.SMTP")
    def test_smtp_diagnostic_requires_tls_and_never_sends(self, factory):
        client = factory.return_value
        client.noop.return_value = (250, b"OK")
        result = connectors.execute("test-smtp", {"config": configuration()})
        self.assertTrue(result["ok"])
        client.starttls.assert_called_once()
        context = client.starttls.call_args.kwargs["context"]
        self.assertTrue(context.check_hostname)
        self.assertEqual(context.verify_mode, ssl.CERT_REQUIRED)
        client.login.assert_called_once_with("sender", "PRIVATE_CONNECTOR_TEST_VALUE")
        client.sendmail.assert_not_called()
        client.data.assert_not_called()

    @patch("connector_io.smtplib.SMTP_SSL")
    def test_implicit_tls_and_submission(self, factory):
        config = configuration()
        config["outgoing"].update(tls="tls", port=465)
        factory.return_value.sendmail.return_value = {}
        result = connectors.execute("send-smtp", {"config": config, "recipient": "recipient@example.org", "message": "From: sender@example.org\r\n\r\nBody"})
        self.assertTrue(result["ok"])
        factory.return_value.starttls.assert_not_called()
        factory.return_value.sendmail.assert_called_once()
        self.assertTrue(factory.call_args.kwargs["context"].check_hostname)

    @patch("connector_io.smtplib.SMTP")
    def test_server_errors_cannot_disclose_secrets(self, factory):
        factory.return_value.login.side_effect = smtplib.SMTPAuthenticationError(535, b"PRIVATE_CONNECTOR_TEST_VALUE server rejection")
        result = connectors.execute("test-smtp", {"config": configuration()})
        self.assertFalse(result["ok"])
        self.assertNotIn("PRIVATE_CONNECTOR_TEST_VALUE", json.dumps(result))
        self.assertNotIn("server rejection", json.dumps(result))

    @patch("connectors.ldap_search")
    def test_ldap_checks_group_and_bases_without_returning_user_information(self, search):
        search.side_effect = ["dn: cn=coordinateurs,ou=groups,dc=example\n", "dn: ou=users,dc=example\n"]
        result = connectors.execute("test-ldap", {"config": configuration()})
        self.assertTrue(result["ok"])
        self.assertEqual(search.call_args.kwargs["scope"], "base")
        self.assertNotIn("dn:", result["message"])
        search.side_effect = [""]
        self.assertFalse(connectors.execute("test-ldap", {"config": configuration()})["ok"])

    @patch("connector_io.subprocess.run")
    def test_ldap_password_uses_private_temporary_file_not_command_line(self, run):
        seen = []
        def inspect(arguments, **kwargs):
            self.assertNotIn("PRIVATE_CONNECTOR_TEST_VALUE", arguments)
            self.assertEqual(kwargs["env"]["LDAPTLS_REQCERT"], "demand")
            filename = arguments[arguments.index("-y") + 1]
            seen.append(filename)
            self.assertEqual(Path(filename).read_text(), "PRIVATE_CONNECTOR_TEST_VALUE")
            self.assertEqual(os.stat(filename).st_mode & 0o777, 0o600)
            return MagicMock(stdout="dn: cn=test\n")
        run.side_effect = inspect
        connector_io.ldap_search("dc=example", "(cn=test)", "dn", config={"url": "ldaps://ldap.example.org", "bind_dn": "cn=service", "bind_password": "PRIVATE_CONNECTOR_TEST_VALUE"})
        self.assertFalse(Path(seen[0]).exists())


if __name__ == "__main__":
    unittest.main()
