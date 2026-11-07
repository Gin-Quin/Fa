"""Provenance of a benchmark executable, checked before reusing it."""

import base64
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


FORMAT = 1


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def file_hash(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def source_hashes(sources):
    return {name: sha256(data) for name, data in sorted(sources.items())}


def source_hash(sources):
    return sha256(json.dumps(source_hashes(sources), sort_keys=True).encode())


def executable_identity(program, version_args=("--version",)):
    resolved = shutil.which(str(program))
    if resolved is None:
        raise RuntimeError(f"compiler not found: {program}")
    # rustc can be a rustup symlink whose invocation name selects the tool.
    path = Path(resolved).absolute()
    version = subprocess.run([str(path), *version_args], capture_output=True, check=False)
    if version.returncode:
        raise RuntimeError(f"cannot identify {path}: its version command failed")
    return {"path": str(path), "real_path": str(path.resolve()), "sha256": file_hash(path),
            "version": (version.stdout + version.stderr).decode(errors="replace").strip()}


def request(name, language, size, sources, compiler, flags, modes, environment=None):
    return {"benchmark": name, "language": language, "size": size,
            "source_sha256": source_hash(sources), "source_files": source_hashes(sources),
            "compiler": compiler, "flags": flags, "modes": modes,
            "environment": environment or {}}


def save(root, manifest_path, requested, binary, arguments, stdout_path, output,
         reference_output=None):
    """Only publish the manifest after compilation and output validation succeed."""
    if reference_output is not None and output != reference_output:
        raise RuntimeError(f"{requested['benchmark']}: {requested['language']} output differs from Rust")
    (root / stdout_path).write_bytes(output)
    manifest = {"format": FORMAT, "request": requested, "binary": binary,
                "binary_sha256": file_hash(root / binary), "arguments": arguments,
                "stdout": stdout_path, "output_sha256": sha256(output),
                "output_base64": base64.b64encode(output).decode("ascii"),
                "reference_output_sha256": sha256(reference_output)
                if reference_output is not None else None}
    destination = root / manifest_path
    temporary = destination.with_suffix(".tmp")
    temporary.write_text(json.dumps(manifest, indent=2) + "\n")
    temporary.replace(destination)
    return manifest


def load(root, manifest_path, requested, binary, arguments, stdout_path,
         reference_output=None):
    """Reject legacy or stale builds even when their old outputs still agree."""
    path = root / manifest_path
    try:
        manifest = json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise RuntimeError(f"{path}: missing or invalid manifest; rebuild without --reuse") from error
    if not isinstance(manifest, dict) or manifest.get("format") != FORMAT:
        raise RuntimeError(f"{path}: unsupported manifest; rebuild without --reuse")
    saved = manifest.get("request")
    if saved != requested:
        fields = sorted(key for key in requested if not isinstance(saved, dict)
                        or saved.get(key) != requested[key])
        raise RuntimeError(f"{path}: build differs in {', '.join(fields) or 'request'}; "
                           "rebuild without --reuse")
    if (manifest.get("binary") != binary or manifest.get("arguments") != arguments
            or manifest.get("stdout") != stdout_path):
        raise RuntimeError(f"{path}: executable command differs; rebuild without --reuse")
    if file_hash(root / binary) != manifest.get("binary_sha256"):
        raise RuntimeError(f"{path}: executable hash differs; rebuild without --reuse")
    try:
        output = base64.b64decode(manifest.get("output_base64", ""), validate=True)
    except (ValueError, TypeError) as error:
        raise RuntimeError(f"{path}: invalid recorded output") from error
    if (sha256(output) != manifest.get("output_sha256")
            or (root / stdout_path).read_bytes() != output):
        raise RuntimeError(f"{path}: saved output differs; rebuild without --reuse")
    expected_reference = sha256(reference_output) if reference_output is not None else None
    if manifest.get("reference_output_sha256") != expected_reference:
        raise RuntimeError(f"{path}: Rust reference differs; rebuild without --reuse")
    if reference_output is not None and output != reference_output:
        raise RuntimeError(f"{path}: output differs from Rust")
    return manifest, output
