#!/usr/bin/env python3
"""Exercise Studio persistence/export, Machine loading, and offline Web playback."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[2]


def run(args, env, *, test=False):
    print("+ " + " ".join(args), flush=True)
    result = subprocess.run(
        args, cwd=REPO, env=env, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=600,
    )
    print(result.stdout, end="", flush=True)
    result.check_returncode()
    if test and "1 passed; 0 failed" not in result.stdout:
        raise RuntimeError("Workflow stage did not execute exactly one passing test")


def main():
    with tempfile.TemporaryDirectory(prefix="caiven-creator-workflow-") as directory:
        root = Path(directory)
        env = dict(os.environ, CAIVEN_WORKFLOW_DIR=directory,
                   APPDATA=str(root / "appdata"))
        # APPDATA takes precedence over HOME for Studio history/token storage.
        # Keep developer preferences and credentials outside this workflow.
        for stage in ("save", "reopen"):
            run(["cargo", "test", "--locked", "-p", "caiven-studio",
                 "tauri_app::creator_workflow::creator_workflow_stage", "--",
                 "--ignored", "--exact", "--nocapture"],
                dict(env, CAIVEN_WORKFLOW_STAGE=stage), test=True)
        for name in ("game.cav", "game.html"):
            if not (root / name).is_file() or not (root / name).stat().st_size:
                raise RuntimeError(f"Missing or empty export: {name}")
        # Playback must succeed without the source project or its module files.
        shutil.rmtree(root / "My first game")
        run(["cargo", "test", "--locked", "-p", "caiven-machine",
             "app::tests::creator_workflow_playback", "--", "--ignored",
             "--exact", "--nocapture"], env, test=True)
        run(["node", "crates/caiven-web/offline_test.mjs",
             str(root / "game.html"), "--creator-workflow"], env)
        print("Creator workflow passed: save, close, fresh reopen, export, Machine and Web playback.")


if __name__ == "__main__":
    main()
