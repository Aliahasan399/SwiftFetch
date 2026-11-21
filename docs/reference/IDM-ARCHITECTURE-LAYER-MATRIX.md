# IDM-Style Download Manager — Layer Matrix

This companion matrix summarizes the full specification in [IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md](IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md).

| Layer | Needed now? | Recommendation |
| --- | --- | --- |
| 1. Presentation/frontend | Yes | Modular desktop UI with queue, progress, scheduler, settings, and accessibility |
| 2. Backend/business logic | Yes | Rust/native core with transfer state machine and scheduler |
| 3. Data/storage | Yes | SQLite/WAL for metadata; filesystem for file bytes |
| 4. Integrations | Yes, limited | Browser extensions, clipboard, drag/drop, OS shell, optional antivirus |
| 5. Security/compliance | Yes | URL/path safety, secret handling, signed updates, no-bypass boundaries |
| 6. DevOps/delivery | Yes, adapted | Desktop CI, fixture server, fault injection, signing, clean-machine tests |
| 7. Local data management | Yes | Durable queue, segment checkpoints, atomic finalization, disk-full recovery |
| 8. Hardware/OS abstraction | Yes | Windows filesystem, power, notifications, proxy, credentials, installer |
| 9. Networking/sync | Yes, limited | HTTP/HTTPS range transfers; no multi-device sync at first |
| 10. Licensing/protection | Later/partial | Signed license optional; avoid invasive anti-debugging and fingerprinting |
| 11. Updates/packaging | Yes for release | Signed installer, rollback, extension packaging, no default telemetry |
| 12. Governance/operations/cost | Yes | Ownership, incident response, legal review, support, cost model |

## Enterprise patterns not required for the first release

- Micro-frontends or server-driven UI
- CRDT collaboration
- Distributed CQRS/event sourcing
- Kafka, RabbitMQ, ESB, GraphQL federation
- Kubernetes, EKS/GKE, ArgoCD, multi-cloud Terraform
- Multi-region active-active replication
- Vector or time-series databases
- OAuth/OIDC/MFA/HSM for a single-user offline app
- GPU acceleration for ordinary downloads
- Custom UDP/TCP framing
- Mandatory online license checks
- Default cloud telemetry

## Product-critical features

- HTTP/HTTPS downloading
- Multi-connection range downloads with safe fallback
- Pause/resume/retry/cancel
- Durable queue and restart recovery
- Scheduler and speed limits
- Browser integration with user consent
- Safe paths, filenames, redirects, cookies, and credentials
- Checksum verification and optional antivirus scan
- Signed installer and rollback-safe updates

