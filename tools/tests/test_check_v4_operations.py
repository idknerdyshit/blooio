import json
import tempfile
import unittest
from pathlib import Path

from tools.check_v4_operations import spec_pairs


class SpecPairsTests(unittest.TestCase):
    def test_separates_planned_operations_and_normalizes_parameters(self):
        document = {
            "paths": {
                "/things/{thingId}": {"get": {}},
                "/things/{thingId}/future": {
                    "post": {"x-blooio-status": "planned"}
                },
            }
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spec.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            implemented, planned = spec_pairs(path)
        self.assertEqual(implemented, {("get", "/things/{parameter}")})
        self.assertEqual(planned, {("post", "/things/{parameter}/future")})


if __name__ == "__main__":
    unittest.main()
