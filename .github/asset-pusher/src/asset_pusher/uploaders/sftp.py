import hashlib
import logging
import os
from pathlib import Path

import paramiko

from asset_pusher import uploaders

logger = logging.getLogger(__name__)


def _hashed_filename(file_path: str) -> str:
    h = hashlib.sha1()
    with open(file_path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    sha1 = h.hexdigest()

    p = Path(file_path)
    suffixes = "".join(p.suffixes)
    stem = p.name[: -len(suffixes)] if suffixes else p.name
    return f"{stem}-{sha1}{suffixes}"


def _sftp_mkdir_p(sftp: paramiko.SFTPClient, remote_path: str) -> None:
    try:
        sftp.stat(remote_path)
        return
    except FileNotFoundError:
        pass

    parent = str(Path(remote_path).parent)
    if parent != remote_path:
        _sftp_mkdir_p(sftp, parent)
    sftp.mkdir(remote_path)


class SftpUploader(uploaders.Uploader):
    def __init__(
        self,
        host: str,
        user: str,
        password: str,
        remote_dir: str,
        http_base_url: str,
        port: int = 22,
        timeout: float = 30.0,
    ) -> None:
        self.host = host
        self.port = port
        self.user = user
        self.password = password
        self.remote_dir = remote_dir.rstrip("/")
        self.http_base_url = http_base_url.rstrip("/")
        self.timeout = timeout

    def upload(self, file_path: str, tag: str) -> uploaders.UploadResult:
        original_name = os.path.basename(file_path)
        remote_filename = _hashed_filename(file_path)
        remote_dir = f"{self.remote_dir}/{tag}"
        remote_path = f"{remote_dir}/{remote_filename}"

        logger.debug("Uploading %s >> sftp://%s:%d", file_path, self.host, self.port)

        with paramiko.SSHClient() as ssh:
            ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
            ssh.connect(
                self.host,
                port=self.port,
                username=self.user,
                password=self.password,
                timeout=self.timeout,
            )
            with ssh.open_sftp() as sftp:
                _sftp_mkdir_p(sftp, remote_dir)
                sftp.put(file_path, remote_path)

        url = f"{self.http_base_url}/{tag}/{remote_filename}"
        return uploaders.UploadResult(
            original_filename=original_name, browser_download_url=url
        )
