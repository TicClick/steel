#!/usr/bin/env python3

import datetime as dt
import glob
import json
import os
import subprocess as sp

RELEASES_JSON = "releases.json"

# "v0.3.51" -> (0, 3, 51)
def to_semver(s):
    return tuple(map(int, s[1:].split(".")))


def asset_info(asset_gist_hash, asset_gist_owner, tagged_commit, filename):
    return {
        "name": filename,
        "size": os.path.getsize(filename),
        "browser_download_url": f"https://gist.githubusercontent.com/{asset_gist_owner}/{asset_gist_hash}/raw/{tagged_commit}/{filename}"
    }


def update_assets_info(asset_gist_hash, asset_gist_owner, metadata_gist_hash, tag_name):
    """
    - Clone assets and get SHA1 of the tagged commit
    - List everything that looks like an asset and compose release info
    - Clone and update releases.json
    """

    sp.check_call(["gh", "gist", "clone", asset_gist_hash])
    os.chdir(asset_gist_hash)

    tagged_commit = sp.check_output(["git", "rev-parse", tag_name]).strip().decode()

    archives = glob.glob("steel-*")
    release_info = {
        "tag_name": tag_name,
        "published_at": dt.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "assets": [
            asset_info(asset_gist_hash, asset_gist_owner, tagged_commit, archive)
            for archive in archives
        ],
    }

    os.chdir("..")

    sp.check_call(["gh", "gist", "clone", metadata_gist_hash])
    os.chdir(metadata_gist_hash)

    with open(RELEASES_JSON, "r") as fd:
        contents = json.load(fd)

    if not contents:
        contents = [release_info]
    else:
        last_release_tag = to_semver(contents[0]["tag_name"])
        current_release_tag = to_semver(tag_name)

        if last_release_tag > current_release_tag:
            print("last release is more fresh, exiting")
            exit(1)

        if last_release_tag == current_release_tag:
            print("release has already been added -- exiting")
            exit(1)

        contents.insert(0, release_info)

    with open(RELEASES_JSON, "w") as fd:
        json.dump(contents, fd)
    
    sp.check_call(["gh", "gist", "edit", metadata_gist_hash, "--filename", RELEASES_JSON, RELEASES_JSON])


def main():
    asset_gist_hash = os.environ["ASSET_GIST_HASH"]
    metadata_gist_hash = os.environ["METADATA_GIST_HASH"]
    tag_name = os.environ["TAG_NAME"]
    asset_gist_owner = os.environ["ASSET_GIST_OWNER"]

    update_assets_info(asset_gist_hash, asset_gist_owner, metadata_gist_hash, tag_name)


if __name__ == "__main__":
    main()
