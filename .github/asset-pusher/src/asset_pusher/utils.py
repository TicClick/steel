import json
import logging
import subprocess as sp
import sys
from collections.abc import Iterable

logger = logging.getLogger(__name__)


def run_with_retries(
    cmd: list,
    *,
    as_json: bool = False,
    errors_to_retry: Iterable[str] = (),
) -> bytes | dict | list:
    cmd = list(map(str, cmd))
    errors_to_retry = list(errors_to_retry)
    for _ in range(10):
        try:
            output = sp.check_output(cmd, stderr=sp.PIPE)
            return json.loads(output) if as_json else output
        except sp.CalledProcessError as e:
            stderr = (e.stderr or b"").decode(errors="replace")
            if errors_to_retry and any(err in stderr for err in errors_to_retry):
                continue
            logger.error("%r failed with stderr: %r", cmd, stderr)
            raise
    logger.error("%r failed after 10 retries", cmd)
    sys.exit(1)
