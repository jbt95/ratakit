#!/usr/bin/env python3
"""Read-only release checks; this script never reserves or publishes crates."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def publishable_packages(metadata: dict) -> list[dict]:
    workspace_ids = set(metadata["workspace_members"])
    return sorted(
        (
            package
            for package in metadata["packages"]
            if package["id"] in workspace_ids
            and package.get("publish") != []
            and (
                package.get("publish") is None
                or "crates-io" in package["publish"]
            )
        ),
        key=lambda package: package["name"],
    )


def dependency_order(packages: list[dict]) -> list[dict]:
    by_name = {package["name"]: package for package in packages}
    dependencies = {
        package["name"]: {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in by_name
        }
        for package in packages
    }
    ordered: list[dict] = []
    ordered_names: set[str] = set()
    ready = sorted(name for name, deps in dependencies.items() if not deps)
    while ready:
        name = ready.pop(0)
        ordered.append(by_name[name])
        ordered_names.add(name)
        for candidate in sorted(dependencies):
            deps = dependencies[candidate]
            if name in deps:
                deps.remove(name)
                if not deps and candidate not in ordered_names and candidate not in ready:
                    ready.append(candidate)
                    ready.sort()
    if len(ordered) != len(packages):
        remaining = sorted(set(by_name) - {package["name"] for package in ordered})
        raise ValueError(f"cyclic local crate dependencies: {', '.join(remaining)}")
    return ordered


def package_files(package: dict) -> set[str]:
    result = subprocess.run(
        ["cargo", "package", "--list", "--allow-dirty", "-p", package["name"]],
        check=True,
        capture_output=True,
        text=True,
    )
    return set(result.stdout.splitlines())


def check(packages: list[dict], version: str) -> list[str]:
    errors: list[str] = []
    for package in packages:
        name = package["name"]
        if package["version"] != version:
            errors.append(f"{name}: expected version {version}, found {package['version']}")
        for field in ("license", "repository", "rust_version"):
            if not package.get(field):
                errors.append(f"{name}: missing package metadata `{field}`")

        files = package_files(package)
        if "Cargo.toml" not in files:
            errors.append(f"{name}: Cargo.toml is missing from package contents")
        if not any(path.startswith("src/") for path in files):
            errors.append(f"{name}: no source files are included in package contents")
        license_file = package.get("license_file")
        if license_file:
            manifest_dir = Path(package["manifest_path"]).parent
            if not (manifest_dir / license_file).is_file():
                errors.append(f"{name}: license file does not exist: {license_file}")
            elif license_file not in files:
                errors.append(f"{name}: license file is not included: {license_file}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--version",
        default="0.1.0",
        help="release version expected in every publishable workspace crate (default: %(default)s)",
    )
    args = parser.parse_args()

    try:
        packages = publishable_packages(cargo_metadata())
        order = dependency_order(packages)
        errors = check(packages, args.version)
    except subprocess.CalledProcessError as error:
        details = (error.stderr or error.stdout or "").strip()
        message = f"{error}\n{details}" if details else str(error)
        print(f"release preflight could not complete: {message}", file=sys.stderr)
        return 2
    except (OSError, ValueError, KeyError) as error:
        print(f"release preflight could not complete: {error}", file=sys.stderr)
        return 2

    print("Publishable crates in dependency order:")
    for index, package in enumerate(order, start=1):
        print(f"  {index}. {package['name']} {package['version']}")
    print("Package contents, license metadata, repository, and MSRV were inspected.")
    print("Run `cargo deny check licenses` for dependency-license policy.")
    print("This command does not reserve names or publish crates.")

    if errors:
        print("\nRelease blockers:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"\nPreflight passed for version {args.version}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
