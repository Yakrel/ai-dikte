from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import ai_dikte_config
import ai_dikte_core


class PlatformScopeTests(unittest.TestCase):
    def test_runtime_owned_fields_are_not_persisted(self) -> None:
        config = ai_dikte_config.build_setup_config(
            {
                "output_driver": "kwtype",
                "hotkey": "something-else",
                "language": "tr-TR",
            }
        )
        self.assertNotIn("output_driver", config)
        self.assertNotIn("hotkey", config)

    def test_kde_wayland_selects_kwtype(self) -> None:
        with mock.patch.object(ai_dikte_core, "IS_WINDOWS", False), \
             mock.patch.object(ai_dikte_core, "desktop_kind", return_value="kde"):
            self.assertEqual(ai_dikte_core.selected_output_driver(), "kwtype")

    def test_hyprland_selects_wtype(self) -> None:
        with mock.patch.object(ai_dikte_core, "IS_WINDOWS", False), \
             mock.patch.object(ai_dikte_core, "desktop_kind", return_value="hyprland"):
            self.assertEqual(ai_dikte_core.selected_output_driver(), "wtype")

    def test_gnome_is_explicitly_unsupported(self) -> None:
        with mock.patch.object(ai_dikte_core, "IS_WINDOWS", False), \
             mock.patch.object(ai_dikte_core, "desktop_kind", return_value="other"):
            with self.assertRaisesRegex(RuntimeError, "Unsupported desktop"):
                ai_dikte_core.selected_output_driver()

    @unittest.skipIf(os.name == "nt", "Linux launcher contract")
    def test_installed_launcher_runs_through_a_profile_symlink(self) -> None:
        source = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory(prefix="ai-dikte install ") as directory:
            root = Path(directory)
            package = root / "store" / "ai-dikte"
            binary = package / "bin" / "ai-dikte"
            library = package / "lib" / "ai-dikte"
            binary.parent.mkdir(parents=True)
            library.mkdir(parents=True)
            shutil.copy2(source / "ai-dikte", binary)
            for name in ("ai_dikte.py", "ai_dikte_core.py", "ai_dikte_config.py"):
                shutil.copy2(source / name, library / name)
            profile = root / "profile" / "bin"
            profile.mkdir(parents=True)
            link = profile / "ai-dikte"
            link.symlink_to(binary)
            direct = subprocess.run([str(binary), "--version"], cwd=root, capture_output=True, text=True)
            linked = subprocess.run([str(link), "--version"], cwd=root, capture_output=True, text=True)
            self.assertEqual(direct.returncode, 0, direct.stderr)
            self.assertEqual(linked.returncode, 0, linked.stderr)
            self.assertEqual(linked.stdout, direct.stdout)


if __name__ == "__main__":
    unittest.main()
