#!/usr/bin/env python3

"""
Upload private builds to put.re and record their metadata for ease of management.
"""

import argparse
import datetime as dt
import json
import logging
import os
import subprocess as sp
import sys

UPLOADS_JSON_FILE = "uploads.json"
RELEASES_JSON_FILE = "releases.json"

logger = logging.getLogger()


def make_asset_info(platform_archive, upload_result):
    return {
        "name": platform_archive,
        "size": os.path.getsize(platform_archive),
        "browser_download_url": upload_result.strip(),
    }


# "v0.3.51" -> (0, 3, 51)
def to_semver(s):
    return tuple(map(int, s[1:].split(".")))


def run_with_retries(cmd=None, as_json=False, errors_to_retry=()):
    cmd = list(map(str, cmd))
    for _ in range(10):
        try:
            output = sp.check_output(cmd)
            return json.loads(output) if as_json else output
        except sp.CalledProcessError as e:
            if any(_ in e.stderr for _ in errors_to_retry):
                continue
            logger.error("%r failed with stderr: %r", cmd, e.stderr)
            raise
    else:
        logger.error("%r failed after 10 retries", cmd)
        sys.exit(1)


def main(platform_archive, uploads_metadata, releases_metadata, release_tag, catbox_hash):
    logger.debug("Uploading %s >> catbox.moe", platform_archive)
    uploaded_file_link = run_with_retries(
        [
            "curl",
            "-F", "reqtype=fileupload",
            "-F", "userhash={}".format(catbox_hash),
            "-F", "fileToUpload=@{}".format(platform_archive),
            "https://catbox.moe/user/api.php",
        ]
    )
    uploaded_file_link = uploaded_file_link.decode()
    platform_archive = os.path.basename(platform_archive)

    if not uploaded_file_link.startswith("https://files.catbox.moe"):
        logger.error("Archive upload failed with output: %r", uploaded_file_link)
        sys.exit(1)

    logger.debug("Fetching gist.github.com >> %s", UPLOADS_JSON_FILE)
    existing_uploads = run_with_retries(
        ["gh", "gist", "view", uploads_metadata, "--filename", UPLOADS_JSON_FILE],
        as_json=True,
        errors_to_retry=("i/o timeout",)
    )
    existing_uploads.setdefault(release_tag, {}).setdefault(platform_archive, uploaded_file_link)
    with open(UPLOADS_JSON_FILE, "w") as fd:
        fd.write(json.dumps(existing_uploads))

    logger.debug("Pushing %s >> gist.github.com", UPLOADS_JSON_FILE)
    run_with_retries(
        ["gh", "gist", "edit", uploads_metadata, "--filename", UPLOADS_JSON_FILE, UPLOADS_JSON_FILE],
        errors_to_retry=("i/o timeout",)
    )

    logger.debug("Fetching gist.github.com >> %s", RELEASES_JSON_FILE)
    existing_releases = run_with_retries(
        ["gh", "gist", "view", releases_metadata, "--filename", RELEASES_JSON_FILE],
        as_json=True,
        errors_to_retry=("i/o timeout",)
    )

    new_asset_info = make_asset_info(platform_archive, uploaded_file_link)
    new_release_info = {
        "tag_name": release_tag,
        "published_at": dt.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "assets": [new_asset_info]
    }

    if not existing_releases:
        existing_releases = [new_release_info]
    else:
        existing_releases.sort(key=lambda r: to_semver(r["tag_name"]))
        last_release = existing_releases[-1]
        last_release_tag = to_semver(last_release["tag_name"])
        current_release_tag = to_semver(release_tag)

        if last_release_tag > current_release_tag:
            logging.error("last release %r > %r current release, exiting", last_release_tag, current_release_tag)
            sys.exit(1)

        elif last_release_tag == current_release_tag:
            if any(
                _["name"] == platform_archive
                for _ in last_release["assets"]
            ):
                logging.warning("release %r has already been added -- overwriting %s", last_release_tag, platform_archive)
            last_release["assets"].append(new_asset_info)

        else:
            existing_releases.append(new_release_info)

    
    with open(RELEASES_JSON_FILE, "w") as fd:
        fd.write(json.dumps(existing_releases))

    logger.debug("Pushing %s >> gist.github.com", RELEASES_JSON_FILE)
    run_with_retries(
        ["gh", "gist", "edit", releases_metadata, "--filename", RELEASES_JSON_FILE, RELEASES_JSON_FILE],
        errors_to_retry=("i/o timeout",)
    )


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--filename", required=True)
    parser.add_argument("--uploads", required=True)
    parser.add_argument("--releases", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--catbox-hash", required=True)
    return parser.parse_args(sys.argv[1:])


if __name__ == "__main__":
    h = logging.StreamHandler(sys.stderr)
    h.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
    logger.addHandler(h)
    logger.setLevel(logging.DEBUG)

    args = parse_args()
    main(
        platform_archive=args.filename,
        uploads_metadata=args.uploads,
        releases_metadata=args.releases,
        release_tag=args.tag,
        catbox_hash=args.catbox_hash,
    )
