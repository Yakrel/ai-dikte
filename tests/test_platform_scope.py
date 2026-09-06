from __future__ import annotations

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

    def test_linux_launchers_do_not_require_usr_bin(self) -> None:
        root = Path(__file__).resolve().parents[1]
        for filename in ("ai-dikte", "ai-dikte-toggle", "ai-dikte.desktop", "ai-dikte-settings.desktop"):
            with self.subTest(filename=filename):
                text = (root / filename).read_text(encoding="utf-8")
                self.assertNotIn("/usr/bin/", text)


if __name__ == "__main__":
    unittest.main()
