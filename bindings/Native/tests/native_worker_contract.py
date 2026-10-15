#!/usr/bin/env python3
"""Actual-library worker queue, ownership and delayed-save contracts."""

import argparse
from pathlib import Path
import sys
import threading

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "examples"))
from native_host import NativeCallError
from native_worker import NativeWorker


def accepted(worker, command):
    result = worker.wait(worker.submit(command))
    assert result["ok"], result
    return result


def position(offset=0):
    return {"section": 0, "paragraph": 0, "scalarOffset": offset, "cellPath": []}


def run(library):
    with NativeWorker(library, {"maxOutstanding": 1}) as worker:
        first = worker.submit({"op": "new", "replace": None})
        try:
            worker.submit({"op": "info"})
        except NativeCallError as error:
            assert error.code == "queueFull"
        else:
            raise AssertionError("unread completion did not reserve queue credit")
        info = worker.wait(first)["info"]
        target = {"documentId": info["documentId"], "revision": info["revision"]}
        original = target.copy()
        selection = {"anchor": position(), "focus": position()}
        accepted(worker, {"op": "beginEdit", "target": target, "selection": selection})
        outcome = accepted(worker, {"op": "replace", "target": target, "text": "가"})["outcome"]
        target["revision"] = outcome["revision"]
        stale = worker.wait(worker.submit({"op": "replace", "target": original, "text": "bad"}))
        assert not stale["ok"] and stale["code"] == "staleRevision"
        size = accepted(worker, {"op": "pageSize", "target": target, "page": 0})
        assert size["size"]["width"] > 0
        caret = accepted(worker, {"op": "caretRect", "target": target, "position": position(1)})
        assert caret["caret"]["height"] > 0

        foreign = []
        def other_thread():
            try:
                worker.host.host_request(op="workerPoll", worker=worker.worker)
            except NativeCallError as error:
                foreign.append(error.code)
        thread = threading.Thread(target=other_thread)
        thread.start()
        thread.join()
        assert foreign == ["invalidArgument"]

        prepared = accepted(worker, {"op": "prepareSave", "target": target, "format": "hwpx"})
        data = worker.copy_buffer(prepared)
        assert data.startswith(b"PK") and prepared["report"]["count"] == 0
        outcome = accepted(worker, {"op": "replace", "target": target, "text": "나"})["outcome"]
        target["revision"] = outcome["revision"]
        saved = accepted(worker, {"op": "acknowledgeSave", "documentId": target["documentId"],
                                  "token": prepared["token"]})
        assert saved["isDirty"]
        request = worker.open_bytes(data, replace=target)
        reopened = worker.wait(request)
        assert reopened["ok"] and reopened["info"]["documentId"] != target["documentId"]
        assert reopened["info"]["revision"] == 0 and not reopened["info"]["isDirty"]
        rejected = worker.wait(worker.submit({"op": "pageSize", "target": target, "page": 0}))
        assert not rejected["ok"] and rejected["code"] == "invalidArgument"

    assert worker.is_finished()
    with NativeWorker(library) as viewer:
        info = accepted(viewer, {"op": "new", "replace": None})["info"]
        target = {"documentId": info["documentId"], "revision": 0}
        frame = accepted(viewer, {"op": "renderRegion", "target": target, "fontGeneration": 0,
                                  "page": 0, "region": {"x": 0, "y": 0, "width": 8, "height": 8},
                                  "scale": 1})
        viewer.close()
        pixels = viewer.copy_buffer(frame)
        assert len(pixels) == frame["height"] * frame["stride"]
        viewer.wait_closed()
    print('{"ok":true,"contract":"native-worker","queueCredit":true,"saveRevision":true,"threadOwnership":true}')


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("library", type=Path)
    run(parser.parse_args().library)
