"""In-situ connector diagnostics and SMTP submission. No raw server error output."""

import imaplib
import json
import smtplib
import ssl
import subprocess
import sys

from connector_io import imap_connection, ldap_search, ldap_escape, smtp_connection


def run(action, payload):
    config = payload["config"]
    if action == "test-imap":
        with imap_connection(config["imap"], readonly=True) as client:
            if client.noop()[0] != "OK":
                raise imaplib.IMAP4.error("IMAP noop failed")
        return "Connexion IMAP TLS et dossier accessibles, UIDPLUS disponible. Aucun message lu, marqué ou supprimé."
    if action == "test-ldap":
        ldap = config["ldap"]
        result = ldap_search(ldap["groups_base"], "(cn=" + ldap_escape(ldap["group"]) + ")", "dn", config=ldap)
        if not any(line.startswith(("dn: ", "dn:: ")) for line in result.splitlines()):
            return None
        # Also check that the configured users base can be queried.
        ldap_search(ldap["users_base"], "(objectClass=*)", "1.1", config=ldap, scope="base")
        return "Annuaire accessible, bases de recherche accessibles et groupe des coordinateurs trouvé. Aucune donnée utilisateur affichée."
    if action == "test-smtp":
        with smtp_connection(config["outgoing"]) as client:
            if client.noop()[0] != 250:
                raise smtplib.SMTPException("SMTP noop failed")
        return "Connexion SMTP TLS établie et authentification acceptée si configurée. Aucun e-mail envoyé."
    if action == "send-smtp":
        smtp = config["outgoing"]
        with smtp_connection(smtp) as client:
            refused = client.sendmail(smtp["sender"], [payload["recipient"]], payload["message"].encode("utf-8"))
            if refused:
                raise smtplib.SMTPRecipientsRefused(refused)
        return "Notification acceptée par le serveur SMTP."
    raise ValueError("Unknown connector action")


def execute(action, payload):
    try:
        message = run(action, payload)
        if message is None:
            return {"ok": False, "message": "Annuaire joignable, mais groupe des coordinateurs introuvable. Vérifiez la base des groupes et le nom du groupe."}
        return {"ok": True, "message": message}
    except ssl.SSLCertVerificationError:
        message = "Certificat TLS refusé : vérifiez le nom du serveur, la validité et la chaîne de certification."
    except smtplib.SMTPAuthenticationError:
        message = "Authentification SMTP refusée. Vérifiez le compte et son secret."
    except imaplib.IMAP4.error:
        message = "Connexion ou authentification IMAP refusée. Vérifiez le compte, son secret et le dossier."
    except subprocess.CalledProcessError:
        message = "Requête LDAP refusée. Vérifiez l’URL, le certificat, les bases et les identifiants éventuels."
    except subprocess.TimeoutExpired:
        message = "Délai de réponse de l’annuaire dépassé."
    except (TimeoutError, OSError):
        message = "Serveur inaccessible ou délai de connexion dépassé. Vérifiez l’adresse, le port et le réseau."
    except (ValueError, KeyError):
        message = "Paramètres incomplets ou incompatibles. IMAP nécessite TLS et UIDPLUS ; SMTP nécessite TLS."
    except smtplib.SMTPException:
        message = "Le serveur SMTP a refusé l’opération. Vérifiez la politique d’envoi et le destinataire."
    except Exception:
        message = "Connecteur indisponible : vérifiez les dépendances et la configuration enregistrée."
    return {"ok": False, "message": message}


if __name__ == "__main__":
    try:
        payload = json.load(sys.stdin)
        result = execute(sys.argv[1], payload)
    except Exception:
        result = {"ok": False, "message": "Configuration du connecteur illisible."}
    print(json.dumps(result))
