# SwiftFetch security and release response

Do not publish active download URLs, request headers, cookies, database backups,
signing keys or other private data in an issue report. Describe the affected
version, action, expected behavior and safe reproduction using a local fixture.
The release owner must provide a private vulnerability-reporting contact before
public distribution.

Downloads are untrusted files. SwiftFetch never opens a downloaded file
automatically. A checksum verifies expected bytes; it is not proof that a file
is safe. Optional antivirus execution uses the configured local scanner and
treats failure or timeout as incomplete scanning.

## Incident response

| Incident | Immediate action | Recovery evidence |
| --- | --- | --- |
| Compromised installer or signing key | Stop distribution and update-channel promotion; revoke affected signing access | Identify affected hashes/versions and distribute a newly reviewed signed release |
| Credential or URL exposure | Stop sharing the affected report/backup; revoke exposed credentials with their issuer | Verify redacted reproduction and repaired logging/export |
| Unexpected overwrite or deletion | Stop affected transfers; preserve the database and files for investigation | Reproduce using disposable fixtures; verify collision/partial ownership tests |
| Browser permission or handoff regression | Disable integration and withdraw the affected package | Review manifest diff and real browser handoff acceptance |
| Incorrect integrity or scanner status | Warn affected users not to execute the file | Reproduce mismatched bytes/scanner failure and verify the fixed state transition |
| Excessive requests or compatibility regression | Pause the affected queue and reduce connections | Check retry/range fixtures and measured request counts |

Local tests and signature checks do not constitute an independent security audit.
Review dependency notices, the release channel, installer identity and actual
permissions before publishing.
