#!/usr/bin/env python3
"""Review runner: serial target matrix, 512 MiB process RSS, saved full logs.

Usage: python3 matrix.py FILE... [--targets bun,native,wasm,native-freestanding]
Logs/artifacts go to /tmp/inline-array-2-matrix. LLVM uses O0/O3 + leak checks.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[4]
OUT = Path('/tmp/inline-array-2-matrix')
OUT.mkdir(exist_ok=True)
ENV = dict(os.environ, PATH=f"{Path.home()}/.bun/bin:{Path.home()}/.local/bin:" + os.environ['PATH'])


def run(command, log, timeout=90):
    with log.open('w') as stream:
        process = subprocess.Popen(['taskset', '-c', '0-4', *command], cwd=ROOT,
                                   env=ENV, stdout=stream, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        start = time.monotonic()
        maximum = 0
        reason = None
        while process.poll() is None:
            try:
                status = Path(f'/proc/{process.pid}/status').read_text()
                rss = next((int(line.split()[1]) for line in status.splitlines()
                            if line.startswith('VmRSS:')), 0)
                maximum = max(maximum, rss)
            except FileNotFoundError:
                pass
            if maximum > 512 * 1024 or time.monotonic() - start > timeout:
                reason = 'RSS limit' if maximum > 512 * 1024 else 'timeout'
                os.killpg(process.pid, signal.SIGKILL)
                break
            time.sleep(0.02)
        code = process.wait()
    return dict(command=command, code=code, rss_kib=maximum, reason=reason,
                log=str(log), output=log.read_text())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('files', nargs='+')
    parser.add_argument('--targets', default='bun,native,wasm,native-freestanding')
    args = parser.parse_args()
    for file in args.files:
        for target in args.targets.split(','):
            for level in ([None] if target in ('bun', 'check') else [0, 3]):
                label = f'{Path(file).stem}-{target}-{level}'
                if target in ('bun', 'check'):
                    cmd = ['target/debug/fa', 'run' if target == 'bun' else 'check', file]
                else:
                    cmd = ['target/debug/examples/build', file, f'--target={target}',
                           f'-O{level}', '--leak-check', f'--output={OUT / label}', '--run']
                result = run(cmd, OUT / f'{label}.log')
                with (OUT / 'results.jsonl').open('a') as stream:
                    stream.write(json.dumps(dict(file=file, target=target, level=level, **result)) + '\n')
                print(label, result['code'], result['rss_kib'], repr(result['output'][:1000]), flush=True)


if __name__ == '__main__':
    main()
