#!/usr/bin/env python3
"""Exercise durable receipt retention through signed REST and PyIceberg calls."""

from __future__ import annotations

import argparse
import copy
import json
import math
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor

import pyiceberg_smoke as smoke


def latency_summary(samples: list[float]) -> dict[str, float | int]:
    if not samples or any(not math.isfinite(value) or value < 0 for value in samples):
        raise ValueError("latency samples must be nonempty, finite, and nonnegative")
    ordered = sorted(samples)
    return {
        "samples": len(ordered),
        "p95_seconds": ordered[math.ceil(len(ordered) * 0.95) - 1],
        "p99_seconds": ordered[math.ceil(len(ordered) * 0.99) - 1],
    }


def run(args: argparse.Namespace, deps: smoke.RuntimeDeps) -> dict[str, object]:
    from pyiceberg.schema import Schema
    from pyiceberg.types import LongType, NestedField

    smoke.ensure_local_proxy_bypass(args.endpoint)
    smoke.ensure_aws_env(args.access_key, args.secret_key, args.region)
    prefix = f"{smoke.normalized_rest_path(args.rest_path)}/v1"
    warehouse = smoke.profile_warehouse(args)
    config = smoke.signed_rest_request(args, deps, "GET", f"{prefix}/config")
    properties = dict(config.get("defaults", {}))
    properties.update(config.get("overrides", {}))
    if properties.get("rustfs.catalog-backing") != "durable-strong":
        raise RuntimeError("this probe requires a preconfigured durable-strong catalog")
    capacity_path = f"{prefix}/{warehouse}/catalog/capacity"
    compact_path = f"{prefix}/{warehouse}/catalog/compact"
    before = smoke.signed_rest_request(args, deps, "GET", capacity_path)
    smoke.signed_rest_request(args, deps, "POST", compact_path)
    catalog = smoke.load_rest_catalog(args, deps)
    names = [args.table, f"{args.table}_peer"]
    created: list[tuple[str, str]] = []
    namespace_created = False
    durations: list[float] = []
    checkpoints = []
    contention: dict[str, object] = {}
    cleanup = "not-requested"

    def path(name: str, suffix: str = "") -> str:
        local = copy.copy(args)
        local.table = name
        return smoke.table_endpoint_path(local, suffix)

    def post(name: str, body: dict[str, object], barrier: threading.Barrier | None = None) -> tuple[int, float]:
        if barrier is not None:
            barrier.wait()
        started = time.monotonic()
        try:
            smoke.signed_rest_request(args, deps, "POST", path(name), body)
            return 200, time.monotonic() - started
        except smoke.RestRequestError as error:
            if error.status_code != 409:
                raise
            return 409, time.monotonic() - started

    try:
        catalog.create_namespace(args.namespace)
        namespace_created = True
        schema = Schema(NestedField(1, "id", LongType(), required=False))
        for name in names:
            table = catalog.create_table((args.namespace, name), schema)
            created.append((args.namespace, name))
            table.append(deps.pyarrow.Table.from_pylist([{"id": 1}, {"id": 2}]))
        first_body = None
        initial = smoke.signed_rest_request(args, deps, "GET", path(names[0]))
        for number in range(40):
            body = {
                "commit-id": f"capacity-{number}",
                "idempotency-key": f"capacity-retry-{number}",
                "requirements": [{"type": "assert-table-uuid", "uuid": initial["metadata"]["table-uuid"]}],
                "updates": [{"action": "set-properties", "updates": {"capacity-probe": str(number)}}],
            }
            status, elapsed = post(names[0], body)
            if status != 200:
                raise RuntimeError("an isolated sequential commit conflicted")
            durations.append(elapsed)
            if first_body is None:
                first_body = copy.deepcopy(body)
            if (number + 1) % 8 == 0:
                checkpoints.append({"commits": number + 1,
                                    "capacity": smoke.signed_rest_request(args, deps, "GET", capacity_path)})
        pointer = smoke.signed_rest_request(args, deps, "GET", path(names[0], "/metadata-location"))
        after = smoke.signed_rest_request(args, deps, "POST", compact_path)
        if after["archived-receipts"] <= before["archived-receipts"]:
            raise RuntimeError("committed history was not archived")
        if post(names[0], first_body)[0] != 200:
            raise RuntimeError("archived commit replay failed")
        conflicting = copy.deepcopy(first_body)
        conflicting["updates"][0]["updates"]["capacity-probe"] = "conflicting-replay"
        if post(names[0], conflicting)[0] != 409:
            raise RuntimeError("changed archived payload was not rejected")
        if smoke.signed_rest_request(args, deps, "GET", path(names[0], "/metadata-location")) != pointer:
            raise RuntimeError("compaction or replay moved the current pointer")

        for scenario, targets in [("same-table", [names[0], names[0]]), ("different-tables", names)]:
            elapsed_samples = []
            conflicts = 0
            for number in range(4):
                requests = []
                ref_name = f"capacity_{scenario.replace('-', '_')}_{number}"
                for writer, name in enumerate(targets):
                    metadata = smoke.signed_rest_request(args, deps, "GET", path(name))["metadata"]
                    requests.append({
                        "commit-id": f"{scenario}-{number}-{writer}",
                        "requirements": [{"type": "assert-ref-snapshot-id", "ref": ref_name, "snapshot-id": None}],
                        "updates": [{"action": "set-snapshot-ref", "ref-name": ref_name,
                                     "snapshot-id": metadata["current-snapshot-id"], "type": "branch"}],
                    })
                with ThreadPoolExecutor(max_workers=2) as pool:
                    barrier = threading.Barrier(2, timeout=10)
                    futures = [pool.submit(post, name, body, barrier) for name, body in zip(targets, requests)]
                    results = [future.result() for future in futures]
                statuses = [status for status, _ in results]
                if scenario == "same-table" and sorted(statuses) != [200, 409]:
                    raise RuntimeError(f"same-base writers did not produce one winner: {statuses}")
                if 200 not in statuses:
                    raise RuntimeError("no writer made progress")
                for name, body, (status, elapsed) in zip(targets, requests, results):
                    if status == 409:
                        conflicts += 1
                        metadata = smoke.signed_rest_request(args, deps, "GET", path(name))["metadata"]
                        body["commit-id"] += "-retry"
                        body["requirements"][0]["snapshot-id"] = metadata.get("refs", {}).get(ref_name, {}).get("snapshot-id")
                        retry_status, retry_elapsed = post(name, body)
                        if retry_status != 200:
                            raise RuntimeError("writer did not progress after serialized reload/retry")
                        elapsed += retry_elapsed
                    elapsed_samples.append(elapsed)
            contention[scenario] = {**latency_summary(elapsed_samples), "initial_conflicts": conflicts}
        for identifier in created:
            rows = catalog.load_table(identifier).scan().to_arrow().to_pylist()
            if sorted(rows, key=lambda row: row["id"]) != [{"id": 1}, {"id": 2}]:
                raise RuntimeError("catalog maintenance changed table data")
        metadata_sizes = {
            name: len(json.dumps(smoke.signed_rest_request(args, deps, "GET", path(name))["metadata"],
                                 separators=(",", ":")).encode("utf-8"))
            for name in names
        }
        final = smoke.signed_rest_request(args, deps, "GET", capacity_path)
    finally:
        if args.cleanup:
            for identifier in reversed(created):
                catalog.drop_table(identifier)
            if namespace_created:
                catalog.drop_namespace(args.namespace)
            cleanup = "catalog-identifiers-dropped; physical-objects-retained"
    return {
        "scope": "single-endpoint-functional-baseline-not-production-slo",
        "sequential": latency_summary(durations), "contention": contention,
        "growth_checkpoints": checkpoints, "current_metadata_json_bytes": metadata_sizes,
        "capacity_before": before, "capacity_after": final, "cleanup": cleanup,
    }


def main() -> int:
    args = smoke.parse_args()
    try:
        result = run(args, smoke.load_runtime_deps())
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
