# RDMA And DPU Acceleration Boundary

This document defines the open-source boundary for future RDMA and DPU
acceleration work. The goal is to keep RustFS behavior-compatible on the default
TCP path while leaving explicit extension points for optional hardware-assisted
data-plane acceleration.

## Open Source Boundary

The open-source build owns stable contracts and safe defaults:

- `TcpHttpInternodeDataTransport` remains the default internode data transport.
- Metadata, lock, health, IAM, KMS, admin, and background controller traffic
  remain on the existing control-plane transports.
- Unknown internode transport values fail closed during configuration parsing.
- The reserved `accelerated-rdma` transport name fails closed in open-source
  builds with an explicit accelerated-backend message.
- S3 RDMA negotiation headers may be detected, but open-source builds must not
  establish an RDMA data path.
- Metrics may report transport fallback decisions, but fallback metrics must use
  low-cardinality labels only.

The open-source path must not require RDMA hardware, DPU devices, kernel bypass
drivers, GPU drivers, vendor SDKs, or external license state.

## Acceleration Boundary

RDMA and DPU acceleration implementations may live behind the existing contracts:

- Implement `InternodeDataTransport` for remote-disk read, write, and walk-dir
  streams.
- Integrate S3 RDMA negotiation after authentication, authorization, request
  validation, and normal S3 compatibility checks.
- Use DPU offload for networking, storage checks, encryption support, telemetry,
  or isolation only when it preserves RustFS object semantics.
- Fall back to the TCP data path when a node, client, device, policy, or
  transfer shape cannot use acceleration safely.

Accelerated data-plane implementations must not bypass quorum, erasure coding,
bitrot, checksum, versioning, retention, IAM, KMS, audit, or bucket policy
semantics.

## Required Invariants

Future RDMA and DPU work must preserve these invariants:

- Object data returned through RDMA and TCP must be byte-identical.
- Write success and failure semantics must match the existing quorum path.
- Retry and fallback behavior must be observable through metrics.
- A mixed cluster must degrade to TCP without changing user-visible S3 behavior.
- No accelerated backend may become the default in open-source builds.
- Vendor-specific code must stay outside default compile and runtime paths.

## Staging Model

The preferred implementation sequence is:

1. Keep contracts, reserved names, and metrics in open-source RustFS.
2. Add accelerated internode RDMA transport behind the transport trait.
3. Add accelerated S3 RDMA negotiation after S3 compatibility and security checks.
4. Add DPU deployment profiles only after RDMA behavior is stable.
5. Certify specific NIC, DPU, driver, and kernel combinations as supported
   acceleration matrices, not open-source requirements.
