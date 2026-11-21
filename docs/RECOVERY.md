# Local queue backup and recovery

SwiftFetch keeps downloads and queue metadata separate. A metadata restore changes
the saved task catalog and settings; it does not delete or replace downloaded
files. Unfinished restored tasks require a manual start.

## Create a backup

Open **Diagnostics → Queue recovery → Back up metadata**. The resulting SQLite
file is saved in the local data folder's `backups` directory. A consistent backup
includes queue rows, resumable checkpoints, categories, schedules and settings.
Keep it private: it contains full URLs, local paths and saved request metadata.
Do not send the database to support or copy it to a public issue.

**Export queue inventory** creates a separate JSON inventory with URLs and request
credentials removed. It still contains local paths. Review those paths before
sharing. An inventory cannot restore authenticated URLs or resumable partial-file
identities.

## Restore a local SQLite backup

1. Select a backup in **Local SQLite backups**, then choose **Restore…**.
2. Review the selected filename and confirm **Restore backup**.
3. SwiftFetch pauses active work, validates a separate staging database and saves
   a consistent safety copy of the current queue before activation.
4. Review the restored task count and the safety-copy path shown in the result.

Backup validation includes SQLite integrity and the application schema. The file
must be in the application's local `backups` folder and within the 128 MiB restore
limit. A failed validation leaves the current queue in place.

Restoration disables saved clipboard capture, browser handoff, antivirus commands,
shutdown requests and schedules. It resets collision handling to Rename and keeps
partial files on failure. Review those settings and enable them again only when
wanted. An online restore preserves the current startup preference and updates
the running catalog. It does not re-register a browser extension or alter Windows
startup registration.

## Recover when the main queue cannot open

SwiftFetch displays a startup recovery page when the queue or saved proxy cannot
initialize. The page shows the data folder and available local SQLite backups.
Choose a backup, explicitly confirm replacement, and select **Validate and restore
backup**. The original database, WAL and SHM files are preserved in a unique safety
folder before activation. Select **Restart and retry** after a successful restore.

If the error is the saved proxy, **Reset saved proxy** clears only that setting.
Confirm the reset, then restart.

If there is no valid backup, preserve the entire data folder and its safety
folders. Keep downloaded files and task-owned partial files in place. A redacted
inventory is useful for recreating a catalog, but it is not a full backup. Do not
delete queue sidecars or partial files as a troubleshooting shortcut.

## Verification and limits

The release verification record lists the automated restore and startup checks
actually completed for each candidate. A local successful restore does not prove
that a damaged disk or a failed operating system can be recovered. Keep an
independent private backup of important data.
