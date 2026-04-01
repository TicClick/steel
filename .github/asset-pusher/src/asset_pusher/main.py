import argparse
import logging
import sys

from asset_pusher.metadata import update_gist_metadata
from asset_pusher.uploaders.catbox import CatboxUploader
from asset_pusher.uploaders.sftp import SftpUploader

logger = logging.getLogger(__name__)


def _setup_logging() -> None:
    h = logging.StreamHandler(sys.stderr)
    h.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
    logging.getLogger().addHandler(h)
    logging.getLogger().setLevel(logging.DEBUG)


def _add_common_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--filename", required=True, help="Path to the file to upload")
    p.add_argument("--tag", required=True, help="Release tag, e.g. v0.9.3")
    p.add_argument("--uploads", required=True, help="Gist ID for uploads.json")
    p.add_argument("--releases", required=True, help="Gist ID for releases.json")
    p.add_argument(
        "--show-upload-url",
        type=bool,
        default=False,
        help="Display URL of the uploaded file (may be sensitive)",
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Upload a private build asset and update release metadata."
    )
    sub = parser.add_subparsers(dest="uploader", required=True)

    catbox = sub.add_parser("catbox", help="Upload to catbox.moe")
    _add_common_args(catbox)
    catbox.add_argument("--catbox-hash", required=True, help="Catbox user hash")

    sftp = sub.add_parser("sftp", help="Upload to an SFTP server (served over HTTP)")
    _add_common_args(sftp)
    sftp.add_argument("--host", required=True, help="SFTP hostname")
    sftp.add_argument("--port", type=int, default=22, help="SFTP port (default: 22)")
    sftp.add_argument("--user", required=True, help="SFTP username")
    sftp.add_argument("--password", required=True, help="SFTP password")
    sftp.add_argument(
        "--remote-dir", required=True, help="Remote base directory on the server"
    )
    sftp.add_argument(
        "--http-base-url",
        required=True,
        help="HTTP base URL corresponding to --remote-dir",
    )
    sftp.add_argument(
        "--timeout",
        type=float,
        default=30.0,
        help="SSH connection timeout in seconds (default: 30)",
    )

    return parser


def main() -> None:
    _setup_logging()
    args = _build_parser().parse_args()

    match args.uploader:
        case "catbox":
            uploader = CatboxUploader(catbox_hash=args.catbox_hash)
        case "sftp":
            uploader = SftpUploader(
                host=args.host,
                port=args.port,
                user=args.user,
                password=args.password,
                remote_dir=args.remote_dir,
                http_base_url=args.http_base_url,
                timeout=args.timeout,
            )
        case _:
            raise ValueError(f"Invalid uploader {args.uploader}")

    result = uploader.upload(args.filename, args.tag)
    if args.show_upload_url:
        logger.debug(
            "Uploaded %s -> %s", result.original_filename, result.browser_download_url
        )

    update_gist_metadata(
        file_path=args.filename,
        name_for_metadata=result.original_filename,
        browser_download_url=result.browser_download_url,
        uploads_gist=args.uploads,
        releases_gist=args.releases,
        release_tag=args.tag,
    )


if __name__ == "__main__":
    main()
