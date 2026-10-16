"""Shared connector operations. Secrets arrive through stdin, never argv/logs."""

import contextlib
import imaplib
import os
import smtplib
import ssl
import subprocess
import tempfile


def ldap_escape(value):
    return "".join("\\%02x" % ord(c) if c in "*()\\\0" else c for c in value)


def ldap_search(base, query, *fields, config=None, scope="sub"):
    config = config or {}
    arguments = ["ldapsearch", "-LLL", "-x", "-H", config.get("url", "ldap://127.0.0.1"),
                 "-b", base, "-s", scope, "-o", "nettimeout=5"]
    environment = dict(os.environ, LDAPTLS_REQCERT="demand")
    with contextlib.ExitStack() as stack:
        if config.get("bind_dn"):
            password = stack.enter_context(tempfile.NamedTemporaryFile(mode="w", encoding="utf-8"))
            password.write(config.get("bind_password", ""))
            password.flush()
            arguments += ["-D", config["bind_dn"], "-y", password.name]
        result = subprocess.run(arguments + [query, *fields], check=True, capture_output=True,
                                text=True, timeout=15, env=environment)
    return result.stdout


@contextlib.contextmanager
def imap_connection(config, readonly=False):
    if not config.get("host") or not config.get("username") or not config.get("password"):
        raise ValueError("IMAP settings incomplete")
    with imaplib.IMAP4_SSL(config["host"], int(config.get("port", 993)),
                          ssl_context=ssl.create_default_context(), timeout=15) as client:
        client.login(config["username"], config["password"])
        if b"UIDPLUS" not in client.capabilities:
            raise ValueError("UIDPLUS required")
        status, _ = client.select(config.get("folder", "INBOX"), readonly=readonly)
        if status != "OK":
            raise imaplib.IMAP4.error("Mailbox unavailable")
        yield client


@contextlib.contextmanager
def smtp_connection(config):
    if not config.get("host"):
        raise ValueError("SMTP settings incomplete")
    context = ssl.create_default_context()
    if config.get("tls") == "tls":
        client = smtplib.SMTP_SSL(config["host"], int(config.get("port", 465)), timeout=15, context=context)
    elif config.get("tls") == "starttls":
        client = smtplib.SMTP(config["host"], int(config.get("port", 587)), timeout=15)
    else:
        raise ValueError("TLS required")
    with client:
        client.ehlo()
        if config.get("tls") == "starttls":
            client.starttls(context=context)
            client.ehlo()
        if config.get("username"):
            if not config.get("password"):
                raise ValueError("SMTP credentials incomplete")
            client.login(config["username"], config["password"])
        yield client
