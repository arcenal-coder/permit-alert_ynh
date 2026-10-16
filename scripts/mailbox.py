"""IMAP TLS ingestion. Sender authorization and aligned DKIM are independent
of untrusted Authentication-Results headers. PDFs are never written to disk.
Requires Debian python3-dkim, python3-dnspython and ldap-utils.
"""

import base64
import email.policy
import email.utils
import hashlib
import json
import os
import re
import sys
from email.parser import BytesParser
from connector_io import imap_connection, ldap_search, ldap_escape

MAX_MESSAGE = 16 * 1024 * 1024
MAX_PDF = 10 * 1024 * 1024


def ldif_values(text, field):
    # Unfold before decoding base64 values (non-ASCII LDAP names/addresses).
    text = text.replace("\n ", "")
    values = []
    for line in text.splitlines():
        if line.startswith(field + ":: "):
            values.append(base64.b64decode(line.split(":: ", 1)[1]).decode())
        elif line.startswith(field + ": "):
            values.append(line.split(": ", 1)[1])
    return values


def authorized_sender(sender, config=None):
    config = config or {}
    group = config.get("group", os.environ.get("PERMIT_ALERT_LDAP_GROUP", "coordinateurs"))
    groups = ldap_search(config.get("groups_base", "ou=groups,dc=yunohost,dc=org"), "(cn=" + ldap_escape(group) + ")", "memberUid", "member", config=config)
    if not ldif_values(groups, "dn"):
        raise RuntimeError("Configured LDAP group missing")
    uids = set(ldif_values(groups, "memberUid"))
    for dn in ldif_values(groups, "member"):
        match = re.match(r"uid=([^,]+)," + re.escape(config.get("users_base", "ou=users,dc=yunohost,dc=org")) + "$", dn, re.I)
        if match:
            uids.add(match[1])
    users = ldap_search(config.get("users_base", "ou=users,dc=yunohost,dc=org"), "(mail=" + ldap_escape(sender) + ")", "uid", config=config)
    return bool(uids.intersection(ldif_values(users, "uid")))


def aligned_dkim(raw, message, sender):
    import dkim
    import dns.resolver

    domain = sender.rsplit("@", 1)[1].encode("ascii")

    def dnsfunc(name, timeout=5):
        answers = dns.resolver.resolve(name.decode().rstrip("."), "TXT", lifetime=timeout)
        return b"".join(answers[0].strings)

    verifier = dkim.DKIM(raw)
    signatures = message.get_all("DKIM-Signature", [])
    for index, signature in enumerate(signatures):
        fields = dict(item.strip().split("=", 1) for item in str(signature).split(";") if "=" in item)
        # Length-limited signatures can leave appended attachments unsigned.
        if "l" in fields:
            continue
        if fields.get("d", "").lower().rstrip(".").encode() != domain.lower():
            continue
        # DKIM verifier checks the signed From header; strict domain alignment.
        try:
            if verifier.verify(idx=index, dnsfunc=dnsfunc):
                return True
        except (dkim.DKIMException, dns.resolver.NXDOMAIN, dns.resolver.NoAnswer, ValueError):
            continue
    return False


def inspect(raw, ldap=None):
    message = BytesParser(policy=email.policy.default).parsebytes(raw)
    from_headers = message.get_all("From", [])
    addresses = email.utils.getaddresses([str(h) for h in from_headers])
    if len(from_headers) != 1 or len(addresses) != 1:
        return {"sender": "inconnu", "reason": "En-tête expéditeur ambigu."}
    sender = addresses[0][1].lower()
    if len(sender) > 254 or not re.fullmatch(r"[^\s<>@]+@[^\s<>@]+\.[^\s<>@]+", sender):
        return {"sender": "inconnu", "reason": "Adresse expéditeur invalide."}
    if not authorized_sender(sender, ldap):
        return {"sender": sender, "reason": "Expéditeur non membre du groupe coordinateurs LDAP."}
    if not aligned_dkim(raw, message, sender):
        return {"sender": sender, "reason": "Signature DKIM alignée sur le domaine expéditeur absente ou invalide."}
    attachments = []
    for part in message.walk():
        if part.get_content_type() == "application/pdf" or str(part.get_filename() or "").lower().endswith(".pdf"):
            content = part.get_payload(decode=True) or b""
            attachments.append(content)
    if len(attachments) != 1:
        return {"sender": sender, "reason": "Un seul PDF E-Permit par e-mail est requis."}
    if not attachments[0].startswith(b"%PDF-") or len(attachments[0]) > MAX_PDF:
        return {"sender": sender, "reason": "Pièce jointe PDF invalide ou supérieure à 10 Mo."}
    return {"sender": sender, "pdf": base64.b64encode(attachments[0]).decode()}


def main():
    config = json.load(sys.stdin)
    imap = config["imap"]
    host = imap["host"]
    with imap_connection(imap) as client:
        validity = client.response("UIDVALIDITY")[1][0].decode()
        if sys.argv[1] == "ack":
            if validity != sys.argv[2] or not sys.argv[3].isdigit():
                raise RuntimeError("Mailbox UID validity changed")
            uid = sys.argv[3]
            if client.uid("STORE", uid, "+FLAGS.SILENT", "(\\Deleted)")[0] != "OK":
                raise RuntimeError("Delete failed")
            if client.uid("EXPUNGE", uid)[0] != "OK":
                raise RuntimeError("Expunge failed")
            print("null")
            return
        status, result = client.uid("SEARCH", None, "UNDELETED")
        if status != "OK":
            raise RuntimeError("Search failed")
        uids = result[0].split()
        if not uids:
            print("null")
            return
        uid = uids[0].decode()
        status, sizes = client.uid("FETCH", uid, "(RFC822.SIZE)")
        if status != "OK":
            raise RuntimeError("Size query failed")
        size = int(re.search(rb"RFC822.SIZE (\d+)", sizes[0]).group(1))
        if size > MAX_MESSAGE:
            event = {"sender": "inconnu", "reason": "E-mail supérieur à 16 Mo."}
        else:
            status, content = client.uid("FETCH", uid, "(BODY.PEEK[])")
            if status != "OK":
                raise RuntimeError("Fetch failed")
            raw = next(item[1] for item in content if isinstance(item, tuple))
            event = inspect(raw, config["ldap"])
        identity = "\0".join([host, str(imap["port"]), imap["username"], imap.get("folder", "INBOX")])
        event.update(uid=uid, validity=validity, mailbox=hashlib.sha256(identity.encode()).hexdigest())
        print(json.dumps(event))


if __name__ == "__main__":
    main()
