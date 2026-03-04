import datetime as dt
import json
import logging
import os
from dataclasses import dataclass

from asset_pusher.utils import run_with_retries

logger = logging.getLogger(__name__)

UPLOADS_JSON_FILE = "uploads.json"
RELEASES_JSON_FILE = "releases.json"
_IO_TIMEOUT = ("i/o timeout",)


@dataclass
class AssetInfo:
    name: str
    size: int
    browser_download_url: str

    def to_dict(self) -> dict:
        return {
            "browser_download_url": self.browser_download_url,
            "name": self.name,
            "size": self.size,
        }


def _to_semver(s: str) -> tuple[int, ...]:
    """'v0.3.51' -> (0, 3, 51)"""
    return tuple(map(int, s[1:].split(".")))


def update_gist_metadata(
    file_path: str,
    name_for_metadata: str,
    browser_download_url: str,
    uploads_gist: str,
    releases_gist: str,
    release_tag: str,
) -> None:
    asset = AssetInfo(
        name=name_for_metadata,
        size=os.path.getsize(file_path),
        browser_download_url=browser_download_url,
    )
    _update_uploads(uploads_gist, release_tag, name_for_metadata, browser_download_url)
    _update_releases(releases_gist, release_tag, asset)


def _update_uploads(gist_id: str, release_tag: str, name: str, url: str) -> None:
    logger.debug("Fetching gist.github.com >> %s", UPLOADS_JSON_FILE)
    existing = run_with_retries(
        ["gh", "gist", "view", gist_id, "--filename", UPLOADS_JSON_FILE],
        as_json=True,
        errors_to_retry=_IO_TIMEOUT,
    )
    existing.setdefault(release_tag, {}).setdefault(name, url)
    with open(UPLOADS_JSON_FILE, "w") as f:
        f.write(json.dumps(existing, sort_keys=True, indent=2))

    logger.debug("Pushing %s >> gist.github.com", UPLOADS_JSON_FILE)
    run_with_retries(
        [
            "gh",
            "gist",
            "edit",
            gist_id,
            "--filename",
            UPLOADS_JSON_FILE,
            UPLOADS_JSON_FILE,
        ],
        errors_to_retry=_IO_TIMEOUT,
    )


def _update_releases(gist_id: str, release_tag: str, asset: AssetInfo) -> None:
    logger.debug("Fetching gist.github.com >> %s", RELEASES_JSON_FILE)
    existing = run_with_retries(
        ["gh", "gist", "view", gist_id, "--filename", RELEASES_JSON_FILE],
        as_json=True,
        errors_to_retry=_IO_TIMEOUT,
    )

    new_release = {
        "assets": [asset.to_dict()],
        "published_at": dt.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "tag_name": release_tag,
    }

    if not existing:
        existing = [new_release]
    else:
        existing.sort(key=lambda r: _to_semver(r["tag_name"]))
        last = existing[-1]
        last_ver = _to_semver(last["tag_name"])
        cur_ver = _to_semver(release_tag)

        if last_ver > cur_ver:
            raise RuntimeError(
                f"Last release {last_ver!r} > {cur_ver!r} current release"
            )
        elif last_ver == cur_ver:
            if any(a["name"] == asset.name for a in last["assets"]):
                logger.warning(
                    "Release %r already has asset %r -- overwriting",
                    last_ver,
                    asset.name,
                )
            last["assets"] = [a for a in last["assets"] if a["name"] != asset.name] + [
                asset.to_dict()
            ]
        else:
            existing.append(new_release)

    with open(RELEASES_JSON_FILE, "w") as f:
        f.write(json.dumps(existing, sort_keys=True, indent=2))

    logger.debug("Pushing %s >> gist.github.com", RELEASES_JSON_FILE)
    run_with_retries(
        [
            "gh",
            "gist",
            "edit",
            gist_id,
            "--filename",
            RELEASES_JSON_FILE,
            RELEASES_JSON_FILE,
        ],
        errors_to_retry=_IO_TIMEOUT,
    )
