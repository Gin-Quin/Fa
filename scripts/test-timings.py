#!/usr/bin/env python3
"""Cargo runner recording per-binary wall time without changing test selection.

FA_SUITE_TIMINGS=/tmp/fa-tests.jsonl cargo test --workspace \
    --config 'target.x86_64-unknown-linux-gnu.runner="scripts/test-timings.py"'

Use an absolute runner path when invoking Cargo outside the workspace root.
Successful-test output is available with Cargo's usual `-- --show-output`.
"""

import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    started = time.monotonic()
    result = subprocess.run(sys.argv[1:], check=False)
    record = {
        "binary": sys.argv[1],
        "seconds": round(time.monotonic() - started, 6),
        "exit": result.returncode,
    }
    if sys.platform == "linux":
        import resource

        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        record.update(max_rss_kib=usage.ru_maxrss, user_seconds=usage.ru_utime,
                      system_seconds=usage.ru_stime)
    destination = os.environ.get("FA_SUITE_TIMINGS")
    if destination:
        # Cargo runs test binaries sequentially. Descendant runners, if any,
        # write a complete JSON line through one append operation.
        data = (json.dumps(record) + "\n").encode()
        with Path(destination).open("ab", buffering=0) as output:
            output.write(data)
    print("FA_SUITE_BINARY " + json.dumps(record), file=sys.stderr)
    return result.returncode if result.returncode >= 0 else 128 - result.returncode


if __name__ == "__main__":
    sys.exit(main())
