from __future__ import annotations

import asyncio
import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


class RuntimeContractTests(unittest.TestCase):
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
        cls.runtime = runpy.run_path(str(Path(__file__).parents[1] / "ai-dikte"))

    @classmethod
    def tearDownClass(cls) -> None:
        cls.environment.stop()
        cls.temp_dir.cleanup()

    def replace_global(self, function_name: str, name: str, value: object) -> None:
        globals_ = self.runtime[function_name].__globals__
        previous = globals_[name]
        globals_[name] = value
        self.addCleanup(globals_.__setitem__, name, previous)

    def test_setup_config_normalizes_consumer_visible_values(self) -> None:
        config = self.runtime["build_setup_config"](
            {},
            {
                "mode": "verbatim",
                "custom_vocabulary": [" Proxmox ", "Proxmox", "AI Dikte"],
                "input_device": "4",
                "audio_cue": False,
                "notify_mode": "none",
                "ui_language": "EN",
            },
        )

        self.assertEqual(config["mode"], "VERBATIM")
        self.assertEqual(config["custom_vocabulary"], ["Proxmox", "AI Dikte"])
        self.assertEqual(config["input_device"], 4)
        self.assertFalse(config["audio_cue"])
        self.assertEqual(config["notify_mode"], "none")
        self.assertEqual(config["ui_language"], "en")

    def test_ui_language_resolution_prefers_explicit_or_defaults_to_turkish(self) -> None:
        resolve = self.runtime["resolve_ui_language"]
        self.assertEqual(resolve({"ui_language": "en"}), "en")
        self.assertEqual(resolve({"ui_language": "tr"}), "tr")
        self.assertEqual(resolve({"ui_language": "auto", "language": "tr-TR"}), "tr")
        self.assertEqual(resolve({"language": "tr-TR"}), "tr")

    def test_config_write_atomically_replaces_file_without_temp_residue(self) -> None:
        directory = Path(tempfile.mkdtemp(dir=self.temp_dir.name))
        config_file = directory / "config.json"
        self.replace_global("write_config", "CONFIG_FILE", config_file)

        self.runtime["write_config"]({"mode": "SMART"})
        self.runtime["write_config"]({"mode": "VERBATIM"})

        self.assertEqual(json.loads(config_file.read_text(encoding="utf-8")), {"mode": "VERBATIM"})
        self.assertEqual(list(directory.iterdir()), [config_file])

    @unittest.skipUnless(os.name == "nt", "Windows Credential Manager contract")
    def test_windows_api_key_stored_exclusively_in_credential_manager(self) -> None:
        directory = Path(tempfile.mkdtemp(dir=self.temp_dir.name))
        config_file = directory / "config.json"
        writes: list[str] = []
        self.replace_global("save_setup_config", "CONFIG_FILE", config_file)
        self.replace_global("save_setup_config", "write_windows_api_key", writes.append)
        self.replace_global("load_api_key", "read_windows_api_key", lambda: "credential-secret")

        key = self.runtime["load_api_key"]()
        self.assertEqual(key, "credential-secret")

        self.runtime["save_setup_config"]("new-secret", {"mode": "SMART"})
        self.assertEqual(writes, ["new-secret"])
        saved_config = json.loads(config_file.read_text(encoding="utf-8"))
        self.assertNotIn("api_key", saved_config)
        self.assertEqual(saved_config["mode"], "SMART")

    @unittest.skipUnless(os.name == "nt", "Windows first-run setup contract")
    def test_windows_first_run_skips_when_credential_manager_has_key(self) -> None:
        wrapper = runpy.run_path(str(Path(__file__).parents[1] / "ai_dikte.py"))
        with mock.patch("sys.platform", "win32"), mock.patch("sys.argv", ["ai-dikte.exe"]):
            with mock.patch("runpy.run_path", return_value={"load_api_key": lambda required=False: "vault-key"}):
                triggered = wrapper["windows_first_run_setup"](Path("ai-dikte"))
                self.assertFalse(triggered)

    @unittest.skipUnless(os.name == "nt", "Windows first-run setup contract")
    def test_windows_first_run_uses_gui_then_starts_daemon(self) -> None:
        wrapper = runpy.run_path(str(Path(__file__).parents[1] / "ai_dikte.py"))
        key_reader = mock.Mock(side_effect=["", "vault-key"])
        gui_mock = mock.Mock()
        daemon_mock = mock.Mock()
        with mock.patch("sys.platform", "win32"), mock.patch("sys.argv", ["ai-dikte.exe"]):
            wrapper["windows_first_run_setup"].__globals__["start_windows_daemon_background"] = daemon_mock
            with mock.patch(
                "runpy.run_path",
                return_value={
                    "load_api_key": key_reader,
                    "run_tray_gui_command": gui_mock,
                },
            ):
                triggered = wrapper["windows_first_run_setup"](Path("ai-dikte"))
                self.assertTrue(triggered)
                gui_mock.assert_called_once_with("setup")
                daemon_mock.assert_called_once()

    @unittest.skipUnless(os.name == "nt", "Windows first-run setup contract")
    def test_windows_first_run_cancel_does_not_start_daemon(self) -> None:
        wrapper = runpy.run_path(str(Path(__file__).parents[1] / "ai_dikte.py"))
        gui_mock = mock.Mock()
        daemon_mock = mock.Mock()
        with mock.patch("sys.platform", "win32"), mock.patch("sys.argv", ["ai-dikte.exe"]):
            wrapper["windows_first_run_setup"].__globals__["start_windows_daemon_background"] = daemon_mock
            with mock.patch(
                "runpy.run_path",
                return_value={
                    "load_api_key": lambda required=False: "",
                    "run_tray_gui_command": gui_mock,
                },
            ):
                triggered = wrapper["windows_first_run_setup"](Path("ai-dikte"))
                self.assertTrue(triggered)
                gui_mock.assert_called_once_with("setup")
                daemon_mock.assert_not_called()
    def test_failed_validation_never_saves_candidate_key(self) -> None:
        save = mock.Mock()
        self.replace_global("setup", "load_config", lambda required=False: {})
        self.replace_global("setup", "load_api_key", lambda required=False: "working-key")
        self.replace_global(
            "setup",
            "getpass",
            SimpleNamespace(getpass=lambda _prompt: "candidate-key"),
        )
        self.replace_global(
            "setup",
            "validate_api_key",
            mock.Mock(side_effect=RuntimeError("invalid key")),
        )
        self.replace_global("setup", "save_setup_config", save)

        with self.assertRaisesRegex(RuntimeError, "invalid key"):
            self.runtime["setup"]()

        save.assert_not_called()

    def test_selected_microphone_reaches_sounddevice_stream(self) -> None:
        captured: dict[str, object] = {}

        class FakeStream:
            def __init__(self, **kwargs: object):
                captured.update(kwargs)

            def start(self) -> None:
                captured["started"] = True

            def stop(self) -> None:
                pass

            def close(self) -> None:
                pass

        recorder_type = self.runtime["SoundDeviceStreamRecorder"]
        self.replace_global(
            "setup",
            "sounddevice",
            SimpleNamespace(RawInputStream=FakeStream),
        )
        loop = asyncio.new_event_loop()
        self.addCleanup(loop.close)
        recorder = recorder_type(device=7)

        recorder.start(loop)
        recorder.stop()

        self.assertEqual(captured["device"], 7)
        self.assertTrue(captured["started"])

    @unittest.skipUnless(os.name == "nt", "Windows Startup contract")
    def test_startup_toggle_configures_and_removes_registry_run_key(self) -> None:
        test_run_name = "AI-Dikte-UnitTest"
        self.replace_global("set_windows_startup_enabled", "REG_RUN_NAME", test_run_name)
        self.replace_global("windows_startup_enabled", "REG_RUN_NAME", test_run_name)

        self.runtime["set_windows_startup_enabled"](True)
        self.assertTrue(self.runtime["windows_startup_enabled"]())

        self.runtime["set_windows_startup_enabled"](False)
        self.assertFalse(self.runtime["windows_startup_enabled"]())
    def test_tray_command_routes_to_platform_entrypoint(self) -> None:
        command = self.runtime["tray_child_command"]("setup")
        expected = "_tray-setup" if os.name == "nt" else "setup"
        self.assertEqual(command[-1], expected)


if __name__ == "__main__":
    unittest.main()
