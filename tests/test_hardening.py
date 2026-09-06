from __future__ import annotations

import importlib
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


class HardeningTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.temp_dir = tempfile.TemporaryDirectory()
        root = Path(cls.temp_dir.name)
        cls.environment = mock.patch.dict(
            os.environ,
            {
                "APPDATA": str(root / "AppData"),
                "LOCALAPPDATA": str(root / "LocalAppData"),
                "XDG_CONFIG_HOME": str(root / "config"),
                "XDG_RUNTIME_DIR": str(root / "runtime"),
            },
        )
        cls.environment.start()
        import ai_dikte_core
        cls.core = importlib.reload(ai_dikte_core)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.environment.stop()
        cls.temp_dir.cleanup()

    def test_linux_notification_failure_is_best_effort(self) -> None:
        with mock.patch.object(self.core, "IS_WINDOWS", False), \
             mock.patch.object(self.core, "load_config", return_value={"notify_mode": "all"}), \
             mock.patch.object(self.core, "log_session_event") as log, \
             mock.patch("shutil.which", return_value="/usr/bin/notify-send"), \
             mock.patch("subprocess.run", side_effect=OSError("dbus unavailable")):
            self.core.notify("AI Dikte", "done")
        self.assertTrue(log.called)

    def test_settings_save_does_not_modify_hyprland_shortcuts(self) -> None:
        existing = self.core.build_setup_config({})
        with mock.patch.object(self.core, "load_config", return_value=existing), \
             mock.patch.object(self.core, "load_api_key", return_value="key"), \
             mock.patch.object(self.core, "validate_api_key") as validate, \
             mock.patch.object(self.core, "save_setup_config") as save, \
             mock.patch.object(self.core, "install_hyprland_shortcut") as shortcut:
            self.core.save_settings("key", dict(existing, notify_mode="none"))
        validate.assert_not_called()
        save.assert_called_once()
        shortcut.assert_not_called()

    def test_hyprland_reload_surfaces_config_errors(self) -> None:
        responses = [
            SimpleNamespace(returncode=0, stdout="", stderr=""),
            SimpleNamespace(returncode=0, stdout="bad lua", stderr=""),
        ]
        with mock.patch("shutil.which", return_value="/usr/bin/hyprctl"), \
             mock.patch("subprocess.run", side_effect=responses):
            with self.assertRaisesRegex(RuntimeError, "configuration has errors"):
                self.core.reload_hyprland()

    def test_doctor_report_contains_unsupported_desktop_failure(self) -> None:
        with mock.patch.object(self.core, "IS_WINDOWS", False), \
             mock.patch.object(self.core, "desktop_kind", return_value="other"):
            report, ok = self.core.doctor_report()
        self.assertFalse(ok)
        self.assertIn("supported-desktop", report)

    def test_linux_fatal_error_notification_is_best_effort(self) -> None:
        import ai_dikte
        with mock.patch.object(ai_dikte.sys, "platform", "linux"), \
             mock.patch.object(ai_dikte.shutil, "which", return_value="/usr/bin/notify-send"), \
             mock.patch.object(ai_dikte.subprocess, "run", side_effect=OSError("no bus")):
            ai_dikte.show_fatal_error("failure")


if __name__ == "__main__":
    unittest.main()
