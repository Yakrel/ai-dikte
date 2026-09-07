from __future__ import annotations

import asyncio
import json
import os
import importlib
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
        import ai_dikte_config, ai_dikte_core
        cls.config = importlib.reload(ai_dikte_config)
        cls.runtime = vars(importlib.reload(ai_dikte_core))

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
        self.assertNotIn("ui_language", config)

    def test_config_write_atomically_replaces_file_without_temp_residue(self) -> None:
        directory = Path(tempfile.mkdtemp(dir=self.temp_dir.name))
        config_file = directory / "config.json"

        with mock.patch.object(self.config, "CONFIG_FILE", config_file):
            self.config.write_config({"mode": "SMART"})
            self.config.write_config({"mode": "VERBATIM"})

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

    def test_first_run_cancel_does_not_start_daemon(self):
        import ai_dikte
        import ai_dikte_core as core
        with mock.patch("sys.platform", "win32"), mock.patch("sys.argv", ["ai-dikte.exe"]), \
             mock.patch.object(ai_dikte, "init_windows_console"), \
             mock.patch.object(core, "load_api_key", return_value=""), \
             mock.patch.object(core, "setup", side_effect=SystemExit(1)), \
             mock.patch.object(core, "main") as dispatch:
            with self.assertRaises(SystemExit):
                ai_dikte.main()
            dispatch.assert_not_called()

    def test_configured_windows_double_click_starts_daemon(self):
        import ai_dikte
        import ai_dikte_core as core
        with mock.patch("sys.platform", "win32"), mock.patch("sys.argv", ["ai-dikte.exe"]), \
             mock.patch.object(ai_dikte, "init_windows_console"), \
             mock.patch.object(ai_dikte, "acquire_windows_daemon_mutex", return_value=True) as mutex, \
             mock.patch.object(core, "load_api_key", return_value="key"), \
             mock.patch.object(core, "setup") as setup, mock.patch.object(core, "main") as dispatch:
            ai_dikte.main()
            mutex.assert_called_once_with("daemon")
            dispatch.assert_called_once()
            setup.assert_not_called()

    def test_failed_validation_never_saves_candidate_key(self) -> None:
        save = mock.Mock()
        self.replace_global("save_settings", "load_config", lambda required=False: {})
        self.replace_global("save_settings", "load_api_key", lambda required=False: "working-key")
        self.replace_global("save_settings", "validate_api_key", mock.Mock(side_effect=RuntimeError("invalid key")))
        self.replace_global("save_settings", "save_setup_config", save)
        with self.assertRaisesRegex(RuntimeError, "invalid key"):
            self.runtime["save_settings"]("candidate-key", {})
        save.assert_not_called()

    def test_local_preferences_save_without_network(self) -> None:
        existing = self.runtime["build_setup_config"]({})
        self.replace_global("save_settings", "load_config", lambda required=False: existing)
        self.replace_global("save_settings", "load_api_key", lambda required=False: "working-key")
        validate, save = mock.Mock(), mock.Mock()
        self.replace_global("save_settings", "validate_api_key", validate)
        self.replace_global("save_settings", "save_setup_config", save)
        candidate = dict(existing, audio_cue=False, notify_mode="none", input_device=3)
        self.runtime["save_settings"]("working-key", candidate)
        validate.assert_not_called()
        save.assert_called_once_with("working-key", candidate)

    def test_audio_preview_is_explicit_and_uses_a_non_silent_wave(self) -> None:
        import io
        import struct
        import wave
        play_sound = mock.Mock()
        fake_winsound = SimpleNamespace(SND_MEMORY=4, SND_NODEFAULT=2, PlaySound=play_sound)
        self.replace_global("play_audio_cue", "IS_WINDOWS", True)
        self.replace_global("play_audio_cue", "load_config", lambda required=False: {"audio_cue": False})
        with mock.patch.dict("sys.modules", {"winsound": fake_winsound}):
            self.runtime["play_audio_cue"]("start")
            play_sound.assert_not_called()
            self.runtime["play_audio_cue"]("start", preview=True)
        payload, _flags = play_sound.call_args.args
        with wave.open(io.BytesIO(payload), "rb") as sound:
            frames = sound.readframes(sound.getnframes())
        samples = struct.unpack(f"<{len(frames) // 2}h", frames)
        self.assertLess(min(samples), 0)
        self.assertGreater(max(samples), 0)

    def test_audio_failure_is_visible_in_preview_but_does_not_abort_feedback(self) -> None:
        fake_winsound = SimpleNamespace(
            SND_MEMORY=4, SND_NODEFAULT=2,
            PlaySound=mock.Mock(side_effect=RuntimeError("no audio device")),
        )
        log = mock.Mock()
        self.replace_global("play_audio_cue", "IS_WINDOWS", True)
        self.replace_global("play_audio_cue", "load_config", lambda required=False: {"audio_cue": True})
        self.replace_global("play_audio_cue", "log_session_event", log)
        with mock.patch.dict("sys.modules", {"winsound": fake_winsound}):
            self.runtime["play_audio_cue"]("stop")
            with self.assertRaisesRegex(RuntimeError, "Windows output device"):
                self.runtime["play_audio_cue"]("start", preview=True)
        self.assertIn("no audio device", log.call_args.args[0])

    def test_feedback_channels_are_independent_and_errors_remain_visible(self) -> None:
        sound, visual = mock.Mock(), mock.Mock()
        config = {}
        self.replace_global("notify", "IS_WINDOWS", True)
        self.replace_global("notify", "load_config", lambda required=False: config)
        self.replace_global("notify", "get_windows_osd", lambda: SimpleNamespace(show=visual))
        fake_winsound = SimpleNamespace(SND_MEMORY=4, SND_NODEFAULT=2, PlaySound=sound)
        with mock.patch.dict("sys.modules", {"winsound": fake_winsound}):
            for audio_enabled, visual_enabled in ((False, False), (False, True), (True, False), (True, True)):
                with self.subTest(audio=audio_enabled, visual=visual_enabled):
                    config.update(audio_cue=audio_enabled, notify_mode="all" if visual_enabled else "none")
                    sound.reset_mock()
                    visual.reset_mock()
                    self.runtime["play_audio_cue"]("start")
                    self.runtime["notify"]("AI Dikte", "Recording started", event_type="start")
                    self.assertEqual(sound.call_count, int(audio_enabled))
                    self.assertEqual(visual.call_count, int(visual_enabled))
                    visual.reset_mock()
                    self.runtime["notify"]("AI Dikte", "Microphone failed", "critical", event_type="error")
                    self.assertIn("Microphone failed", visual.call_args.args[0])

    def test_api_preferences_validate_before_saving(self) -> None:
        existing = self.runtime["build_setup_config"]({})
        self.replace_global("save_settings", "load_config", lambda required=False: existing)
        self.replace_global("save_settings", "load_api_key", lambda required=False: "key")
        calls = mock.Mock()
        self.replace_global("save_settings", "validate_api_key", calls.validate)
        self.replace_global("save_settings", "save_setup_config", calls.save)
        for updates in ({"language": "en-US"}, {"mode": "VERBATIM"}, {"custom_vocabulary": ["Proxmox"]}):
            with self.subTest(updates=updates):
                calls.reset_mock()
                candidate = dict(existing, **updates)
                self.runtime["save_settings"]("key", candidate)
                self.assertEqual(calls.mock_calls, [mock.call.validate("key", candidate), mock.call.save("key", candidate)])

    def test_windows_setup_uses_gui_without_taking_an_outer_lock(self) -> None:
        gui = mock.Mock(return_value=True)
        self.replace_global("main", "IS_WINDOWS", True)
        self.replace_global("main", "ensure_runtime", mock.Mock())
        self.replace_global("main", "FileLock", mock.Mock(side_effect=AssertionError("outer lock")))
        self.replace_global("main", "run_tray_gui_command", gui)
        with mock.patch("sys.argv", ["ai-dikte", "setup"]):
            self.runtime["main"]()
        gui.assert_called_once_with("setup")
        gui.return_value = False
        with self.assertRaises(SystemExit) as raised:
            self.runtime["setup"]()
        self.assertEqual(raised.exception.code, 1)

    def test_linux_setup_uses_the_same_gui(self):
        gui = mock.Mock(return_value=True)
        self.replace_global("setup", "IS_WINDOWS", False)
        self.replace_global("setup", "run_tray_gui_command", gui)
        self.runtime["setup"]()
        gui.assert_called_once_with("setup")

    def test_typing_failure_does_not_try_another_backend(self):
        self.replace_global("output_text", "IS_WINDOWS", False)
        self.replace_global("output_text", "desktop_kind", lambda: "hyprland")
        with mock.patch("shutil.which", return_value="/usr/bin/wtype"), \
             mock.patch("subprocess.run", return_value=SimpleNamespace(returncode=1, stderr="failed")) as run:
            with self.assertRaisesRegex(RuntimeError, "wtype failed"):
                self.runtime["output_text"]("hello")
            run.assert_called_once()

    def test_missing_desktop_backend_does_not_select_installed_alternative(self):
        self.replace_global("selected_output_driver", "IS_WINDOWS", False)
        self.replace_global("selected_output_driver", "desktop_kind", lambda: "hyprland")
        with mock.patch("shutil.which", side_effect=lambda name: "/usr/bin/kwtype" if name == "kwtype" else None):
            self.assertIsNone(self.runtime["available_output_driver"]())

    def test_linux_daemon_fails_explicitly(self):
        self.replace_global("run_daemon", "IS_WINDOWS", False)
        with self.assertRaisesRegex(RuntimeError, "Windows-only"):
            self.runtime["run_daemon"]()

    def test_linux_session_recognizes_python_entrypoint(self):
        self.replace_global("session_matches", "IS_WINDOWS", False)
        self.replace_global("session_matches", "pid_alive", lambda _pid: True)
        cmdline = b"/usr/bin/python\0/usr/lib/ai-dikte/ai_dikte.py\0_live-session\0"
        with mock.patch.object(Path, "read_bytes", return_value=cmdline):
            self.assertTrue(self.runtime["session_matches"](1234))

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
        expected = "_tray-setup"
        self.assertEqual(command[-1], expected)


class RecordingBoundaryTests(unittest.IsolatedAsyncioTestCase):
    async def run_stopped_session(self, audio: bytes) -> list[dict]:
        import ai_dikte_core as core
        recorder = core.SoundDeviceStreamRecorder()
        messages = []
        ended = asyncio.Event()

        async def send(raw):
            message = json.loads(raw)["realtimeInput"]
            messages.append(message)
            if "activityStart" in message and audio:
                # PortAudio has delivered the last block, but its event-loop
                # callback has not run when the user requests stop.
                recorder._callback(audio, len(audio) // 2, None, None)
            if "activityEnd" in message:
                ended.set()

        async def receive(_websocket, transcripts, complete):
            await ended.wait()
            await transcripts.put(("final", "Recorded."))
            complete.set()

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.multiple(core,
                IS_WINDOWS=True, websockets=SimpleNamespace(),
                RUNTIME_DIR=root, SESSION_LOG=root / "session.log",
                READY_FILE=root / "ready", ERROR_FILE=root / "error",
                load_config=lambda: {}, load_api_key=lambda: "key",
                notify=mock.Mock(), play_audio_cue=mock.Mock(),
                log_session_event=mock.Mock(), clear_runtime_state=mock.Mock(),
                SoundDeviceStreamRecorder=lambda **_kwargs: recorder,
                sounddevice=SimpleNamespace(RawInputStream=mock.Mock(return_value=mock.Mock())),
                open_live_websocket=mock.AsyncMock(return_value=SimpleNamespace(
                    send=send, close=mock.AsyncMock())),
                receive_transcriptions=receive, output_text=mock.Mock(return_value="sendinput"),
            ):
                await core.live_session(stop_checker=lambda: True)
        return messages

    async def test_stop_flushes_audio_already_delivered_by_portaudio(self):
        import base64
        audio = b"\x12\x34" * 1600
        messages = await self.run_stopped_session(audio)
        sent = b"".join(base64.b64decode(m["audio"]["data"]) for m in messages if "audio" in m)
        self.assertEqual(sent, audio)

    async def test_empty_capture_fails_instead_of_sending_synthetic_silence(self):
        with self.assertRaisesRegex(RuntimeError, "No audio was captured"):
            await self.run_stopped_session(b"")


class TranscriptTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        import ai_dikte_core
        self.runtime = vars(ai_dikte_core)
        self.log_patch = mock.patch.object(ai_dikte_core, "log_session_event")
        self.log_patch.start()
        self.addCleanup(self.log_patch.stop)
        self.queue = asyncio.Queue()
        self.complete = asyncio.Event()
        self.receiver = asyncio.create_task(asyncio.sleep(60))

    async def asyncTearDown(self) -> None:
        self.receiver.cancel()
        try:
            await self.receiver
        except asyncio.CancelledError:
            pass

    async def collect(self, timeout=3):
        return await self.runtime["collect_final_transcript"](
            self.queue, self.receiver, self.complete, timeout=timeout
        )

    async def test_distinct_finals_keep_repeated_and_overlapping_words(self):
        for text in ("Evet.", "Evet.", "Bir", "Bir daha", "daha"):
            await self.queue.put(("final", text))
        self.complete.set()
        self.assertEqual(await self.collect(), "Evet. Evet. Bir Bir daha daha")

    async def test_waits_for_delayed_tail_after_old_quiet_cutoff(self):
        await self.queue.put(("final", "First."))
        await self.queue.put(("interim", "Second"))
        collecting = asyncio.create_task(self.collect())
        await asyncio.sleep(0.85)
        self.assertFalse(collecting.done())
        self.complete.set()
        await asyncio.sleep(0.6)
        self.assertFalse(collecting.done())
        await self.queue.put(("final", "Second."))
        self.assertEqual(await collecting, "First. Second.")

    async def test_timeout_does_not_silently_drop_pending_interim(self):
        await self.queue.put(("final", "First."))
        await self.queue.put(("interim", "unfinished"))
        self.complete.set()
        with self.assertRaisesRegex(RuntimeError, "Timed out"):
            await self.collect(timeout=0.15)

    async def test_disconnect_without_completion_does_not_emit_partial_text(self):
        await self.queue.put(("final", "First."))
        self.receiver.cancel()
        try:
            await self.receiver
        except asyncio.CancelledError:
            pass
        self.receiver = asyncio.create_task(asyncio.sleep(0))
        await self.receiver
        with self.assertRaisesRegex(RuntimeError, "before transcription was complete"):
            await self.collect()

    async def test_final_can_arrive_just_after_completion(self):
        self.complete.set()
        collecting = asyncio.create_task(self.collect())
        await asyncio.sleep(0.1)
        await self.queue.put(("final", "Late final."))
        self.assertEqual(await collecting, "Late final.")


if __name__ == "__main__":
    unittest.main()
