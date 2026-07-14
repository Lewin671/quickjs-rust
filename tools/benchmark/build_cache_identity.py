"""Effective hosted build and system identity inputs for executable caches."""

from __future__ import annotations

import hashlib
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

from .schema import sha256_file


_BUILD_ENV_NAMES = {
    "AR", "ARFLAGS", "CC", "CFLAGS", "CMAKE_GENERATOR", "CPPFLAGS", "CXX",
    "CXXFLAGS", "LD", "LDFLAGS", "MAKEFLAGS", "NM", "OBJCOPY", "RANLIB",
    "STRIP", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET", "BINDGEN_EXTRA_CLANG_ARGS",
    "LIBCLANG_PATH", "PKG_CONFIG_PATH", "PKG_CONFIG_LIBDIR",
    "PKG_CONFIG_SYSROOT_DIR", "RUSTC", "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS", "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTFLAGS", "CARGO_BUILD_TARGET",
}
_CARGO_TARGET_ENV = re.compile(r"CARGO_TARGET_[A-Z0-9_]+_(?:LINKER|RUSTFLAGS|RUNNER)\Z")
_CARGO_PROFILE_ENV = re.compile(r"CARGO_PROFILE_[A-Z0-9_]+\Z")
_TARGET_C_ENV = re.compile(
    r"(?:CC|CXX|AR|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS|BINDGEN_EXTRA_CLANG_ARGS)_.+\Z",
    re.IGNORECASE,
)


class BuildIdentityError(ValueError):
    """A configured build or system identity cannot be inspected."""


def run(command: list[str], *, cwd: Path | None = None) -> str:
    result = subprocess.run(
        command, cwd=cwd, capture_output=True, text=True, timeout=30, check=False
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise BuildIdentityError(f"cannot inspect build input with {command[0]}: {detail}")
    return result.stdout.strip()


def tool(command: str) -> str:
    resolved = shutil.which(command)
    if resolved is None:
        raise BuildIdentityError(f"required build tool is unavailable: {command}")
    # Multicall shims such as rustup dispatch from argv[0].
    return str(Path(resolved).absolute())


def configured_tool(value: str, fallback: str) -> str:
    command = value or fallback
    resolved = shutil.which(command)
    if resolved is None:
        path = Path(command).expanduser()
        if not path.is_file():
            raise BuildIdentityError(f"configured build tool is unavailable: {command}")
        resolved = str(path)
    return str(Path(resolved).absolute())


def effective_build_environment() -> dict[str, str]:
    """Return inherited variables that may alter compiled or linked output."""
    return {
        name: value
        for name, value in sorted(os.environ.items())
        if name in _BUILD_ENV_NAMES
        or _CARGO_TARGET_ENV.fullmatch(name)
        or _CARGO_PROFILE_ENV.fullmatch(name)
        or _TARGET_C_ENV.fullmatch(name)
        or name.startswith("CMAKE_")
    }


def cargo_configuration_identity() -> dict[str, Any]:
    """Hash global Cargo configuration that can select compilers or linkers."""
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).expanduser()
    configs = []
    for name in ("config", "config.toml"):
        path = cargo_home / name
        if path.is_file():
            configs.append({"path": str(path.absolute()), "sha256": sha256_file(path)})
    return {"cargo_home": str(cargo_home.absolute()), "configs": configs}


def version_identity(command: str) -> dict[str, str]:
    path = configured_tool(command, command)
    result = subprocess.run(
        [path, "--version"], capture_output=True, text=True, timeout=10, check=False
    )
    rendered = (result.stdout + result.stderr).strip()
    return {
        "path": path,
        "sha256": sha256_file(Path(path)),
        "version": rendered if result.returncode == 0 and rendered else "unavailable",
    }


def system_identity() -> dict[str, Any]:
    """Bind hosted image, kernel, libc, and default system linker identity."""
    os_release = Path("/etc/os-release")
    os_release_sha256 = (
        hashlib.sha256(os_release.read_bytes()).hexdigest()
        if os_release.is_file() else "unavailable"
    )
    ldd = shutil.which("ldd")
    libc = {
        "platform": list(platform.libc_ver()),
        "ldd_path": str(Path(ldd).absolute()) if ldd else "unavailable",
        "ldd_identity": run([ldd, "--version"]) if ldd else "unavailable",
    }
    cc = tool("cc")
    linker_name = run([cc, "-print-prog-name=ld"])
    linker = configured_tool(linker_name, "ld")
    return {
        "runner_image": {
            "os": os.environ.get("ImageOS", "unavailable"),
            "version": os.environ.get("ImageVersion", "unavailable"),
        },
        "platform": {
            "system": platform.system(), "machine": platform.machine(),
            "release": platform.release(), "version": platform.version(),
        },
        "os_release_sha256": os_release_sha256,
        "libc": libc,
        "default_linker": {
            "driver": version_identity(cc),
            "linker": version_identity(linker),
        },
    }


def rust_linker_identity(target: str, environment: dict[str, str]) -> dict[str, Any]:
    target_key = "CARGO_TARGET_" + re.sub(r"[^A-Za-z0-9]", "_", target).upper() + "_LINKER"
    configured = environment.get(target_key, "")
    if configured:
        return {"source": target_key, **version_identity(configured)}
    return {"source": "rust_default_cc_driver", **system_identity()["default_linker"]}


def configured_tool_identities(environment: dict[str, str]) -> dict[str, dict[str, str]]:
    names = {
        name for name in environment
        if name in {
            "AR", "CC", "CXX", "LD", "NM", "OBJCOPY", "RANLIB", "STRIP",
            "RUSTC", "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTC_WRAPPER",
        } or name.endswith("_LINKER")
        or re.fullmatch(r"(?:CC|CXX|AR)_.+", name, re.IGNORECASE)
    }
    return {
        name: version_identity(environment[name])
        for name in sorted(names) if environment[name]
    }
