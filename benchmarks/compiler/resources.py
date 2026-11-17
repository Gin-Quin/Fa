"""Fresh-process CPU/wall and post-exec RSS, independent of parent high water."""
import hashlib
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


def sample(command, expected, timeout=300, max_rss_kib=None, *, allowed_exits=(0,), check=True, output_path=None):
    """GNU time forks after exec, so its child RSS excludes Python peak RSS.

    wait4 retains microsecond CPU precision for the launcher and waited child;
    this includes the small, identical GNU-time overhead for both tools.
    """
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        rss_file = directory / "rss"
        with (directory / "stdout").open("wb") as stdout, (directory / "stderr").open("wb") as stderr:
            load_start = os.getloadavg()[0]
            started = time.perf_counter()
            child = subprocess.Popen(["/usr/bin/time", "-q", "-f", "%M", "-o", str(rss_file), *command],
                                     stdout=stdout, stderr=stderr, start_new_session=True)
            failure = None
            tool_pid = None
            observed_peak = 0
            while True:
                pid, status, usage = os.wait4(child.pid, os.WNOHANG)
                if pid:
                    child.returncode = os.waitstatus_to_exitcode(status)
                    break
                try:
                    if tool_pid is None:
                        children = Path(f"/proc/{child.pid}/task/{child.pid}/children").read_text().split()
                        tool_pid = int(children[0]) if children else None
                    proc = Path(f"/proc/{tool_pid}/status").read_text() if tool_pid else ""
                    rss = next((int(line.split()[1]) for line in proc.splitlines()
                                if line.startswith("VmRSS:")), 0)
                    observed_peak = max(observed_peak, rss)
                    if max_rss_kib is not None and rss > max_rss_kib:
                        failure = f"resident memory exceeded {max_rss_kib} KiB"
                except (FileNotFoundError, ProcessLookupError):
                    pass
                if time.perf_counter() - started > timeout:
                    failure = f"timeout after {timeout}s"
                if failure:
                    os.killpg(child.pid, signal.SIGKILL)
                time.sleep(.002)
        wall = time.perf_counter() - started
        if child.returncode not in allowed_exits and failure is None:
            failure = f"exit {child.returncode}"
        peak_text = rss_file.read_text().strip() if rss_file.exists() else ""
        peak = int(peak_text) if peak_text.isdigit() else observed_peak
        if max_rss_kib is not None and peak > max_rss_kib:
            failure = f"peak {peak} KiB exceeded {max_rss_kib} KiB"
        stdout_bytes = (directory / "stdout").read_bytes()
        diagnostics = stdout_bytes + (directory / "stderr").read_bytes()
        if output_path is not None:
            output_path.write_bytes(diagnostics)
        if failure and check:
            raise RuntimeError(f"{command}: {failure}: " + diagnostics.decode(errors="replace")[-2000:])
        if expected is not None and stdout_bytes != expected:
            raise RuntimeError(f"{command}: unexpected stdout")
        return {"load_1m_start": load_start, "load_1m_end": os.getloadavg()[0],
                "wall": wall, "user": usage.ru_utime, "system": usage.ru_stime,
                "cpu": usage.ru_utime + usage.ru_stime, "peak_rss_kib": peak,
                "launcher_peak_rss_kib": usage.ru_maxrss, "exit": child.returncode,
                "failure": failure, "diagnostic_bytes": len(diagnostics),
                "diagnostic_sha256": hashlib.sha256(diagnostics).hexdigest(),
                "rss_method": "GNU time child" if peak_text.isdigit() else "observed before kill"}
