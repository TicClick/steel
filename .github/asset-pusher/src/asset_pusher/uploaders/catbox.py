import logging
import os

from asset_pusher.uploaders import Uploader, UploadResult
from asset_pusher.utils import run_with_retries

logger = logging.getLogger(__name__)


class CatboxUploader(Uploader):
    def __init__(self, catbox_hash: str) -> None:
        self.catbox_hash = catbox_hash

    def upload(self, file_path: str, tag: str) -> UploadResult:
        logger.debug("Uploading %s >> catbox.moe", file_path)
        url = (
            run_with_retries(
                [
                    "curl",
                    "-F",
                    "reqtype=fileupload",
                    "-F",
                    f"userhash={self.catbox_hash}",
                    "-F",
                    f"fileToUpload=@{file_path}",
                    "https://catbox.moe/user/api.php",
                ]
            )
            .decode()
            .strip()
        )

        if not url.startswith("https://files.catbox.moe"):
            raise RuntimeError(f"Catbox upload failed: {url!r}")

        return UploadResult(
            original_filename=os.path.basename(file_path),
            browser_download_url=url,
        )
