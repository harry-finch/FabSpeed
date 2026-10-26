# Windows publisher — implementation boundary

FINAL-2 preserves the Windows refusal. No native Windows machine is available in this run; an unexecuted ACL/replace implementation must not be enabled. The coordinator, consent, projection, validation, generations and recovery remain shared. A new early guard also refuses settings and manual publication before writing controls, including disabled settings with no destination. Native guard regression is in `publication::windows_guard_tests`.

## Required backend, still blocked

The current IO surface is `publication_fs::{destination,lease,read,atomic_private,sync_dir,file_identity,block,is_blocked}` plus temporary creation/replacement in `publication::attempt` and source creation/replacement in `source`. All three durable records must use the same tested backend; replacing only the public-file rename is insufficient.

A Windows implementation must restrict the support contract to positively identified local NTFS volumes, reject UNC/device/alternate-stream paths and each reparse-point component, open handles without following reparse points, compare final volume/file identities, reject multiple links, and inspect owner SID and protected DACL. Temporary files and the destination lease need explicit current-user-only ACLs at creation, not inherited broad access. The existing OS lease can remain, but Windows share modes and locked-file behavior need native tests.

Candidate replacement primitives require an explicit design choice. Microsoft documents [ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew) with ACL/stream inheritance and partial-failure states (1175/1176/1177); its WRITE_THROUGH flag is unsupported. Therefore it is not a drop-in equivalent of the current Unix protocol. [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) has different same-volume and write-through semantics; never allow copy-across-volume or delete-first fallbacks. Each failure must be reconciled by actual known file identity and complete byte digest, never by the return code alone.

[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) establishes file flushing, not an independently proven POSIX directory-fsync contract. [Directory handles](https://learn.microsoft.com/en-us/windows/win32/fileio/obtaining-a-handle-to-a-directory) require BACKUP_SEMANTICS; this does not prove directory durability. No administrative volume-flush privilege is to be requested. Logical visibility, file persistence, metadata persistence and power-loss guarantees must be qualified separately.

Withdrawal needs a protected DACL on a positively identified owned file, retaining only the rights required to replace it later while removing ordinary read access. Owner/root-equivalent processes and already-open handles/copies cannot be revoked. Never translate readonly attributes into a false chmod-000 promise. If restriction cannot be proved, report the old content as potentially readable and keep disable intent; require operator intervention.

## Fixed native acceptance gate

Use the same coordinator tests with a Windows fixture backend, plus NTFS owner/DACL, junction/reparse, hard-link, open reader with and without FILE_SHARE_DELETE, antivirus/sharing violation, unknown destination, parent permissions, concurrent writers and cross-volume refusal. Run actual child-process termination before reservation, after reservation, during write, before/after replacement, after source commit, and after withdrawal. Replay produced JSON with the independent Python reader. Add a case for each partial-replacement error disposition. Neither compilation nor a mocked Win32 call is native PASS.

The dedicated `native-qualification.yml` prepares non-publishing Windows/Linux gates. It must only be dispatched after separately authorized remote execution. Until the backend and these tests pass, Windows publication and Windows V1 acceptance remain BLOCKED; existing calculations/library functions are not claimed qualified by this document.
