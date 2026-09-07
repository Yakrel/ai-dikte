"""Real widget scenarios run on Windows and under Xvfb in Linux CI."""
import os
import unittest
from contextlib import nullcontext
from unittest import mock
from ai_dikte_config import build_setup_config
from ai_dikte_ui import SettingsServices, show_dialog


@unittest.skipUnless(os.name == "nt" or os.environ.get("DISPLAY"), "GUI display required")
class DialogTests(unittest.TestCase):
    def run_dialog(self, services, interact):
        import tkinter as tk
        root = tk.Tk()
        errors = []

        def descendants(widget):
            for child in widget.winfo_children():
                yield child
                yield from descendants(child)

        def run_interaction():
            try:
                interact(root, list(descendants(root)))
            except Exception as exc:
                errors.append(exc)
                root.destroy()

        root.after(100, run_interaction)
        root.after(5000, root.destroy)
        with mock.patch("tkinter.Tk", return_value=root):
            result = show_dialog("setup", services)
        self.assertEqual(errors, [])
        return result

    def test_save_cancel_and_platform_controls(self):
        for windows_controls in (False, True):
            for action in ("Save", "Cancel"):
                with self.subTest(windows=windows_controls, action=action):
                    save, startup = mock.Mock(), mock.Mock()
                    services = SettingsServices(
                        load_config=lambda: build_setup_config({}),
                        load_key=lambda: "test-key", save=save,
                        diagnostics=lambda: ("OK", True), lock=nullcontext,
                        preview_audio=mock.Mock() if windows_controls else None,
                        devices=(lambda: [(None, "System default")]) if windows_controls else None,
                        get_startup=(lambda: False) if windows_controls else None,
                        set_startup=startup if windows_controls else None,
                    )

                    def interact(_root, controls):
                        startup_widgets = [w for w in controls if "text" in w.keys() and w.cget("text") == "Start with Windows"]
                        self.assertEqual(bool(startup_widgets), windows_controls)
                        next(w for w in controls if w.winfo_class() == "TButton" and w.cget("text") == action).invoke()

                    self.assertEqual(self.run_dialog(services, interact), action == "Save")
                    if action == "Save":
                        save.assert_called_once()
                        if windows_controls:
                            startup.assert_called_once_with(False)
                        else:
                            self.assertIsNone(save.call_args.args[1]["input_device"])
                            startup.assert_not_called()
                    else:
                        save.assert_not_called()
                        startup.assert_not_called()

    def test_notification_preferences_are_independent_and_survive_reopening(self):
        for sound, visual in ((False, False), (False, True), (True, False), (True, True)):
            with self.subTest(sound=sound, visual=visual):
                config = build_setup_config({})
                services = SettingsServices(
                    load_config=lambda: dict(config), load_key=lambda: "test-key",
                    save=lambda _key, candidate: config.update(candidate),
                    diagnostics=lambda: ("OK", True), lock=nullcontext,
                    preview_audio=mock.Mock(),
                )

                def interact(_root, controls, *, saving):
                    for prefix, expected in (("Play audio", sound), ("Show visual", visual)):
                        widget = next(w for w in controls if w.winfo_class() == "TCheckbutton" and w.cget("text").startswith(prefix))
                        selected = widget.instate(["selected"])
                        if saving and selected != expected:
                            widget.invoke()
                        elif not saving:
                            self.assertEqual(selected, expected)
                    action = "Save" if saving else "Cancel"
                    next(w for w in controls if w.winfo_class() == "TButton" and w.cget("text") == action).invoke()

                self.assertTrue(self.run_dialog(services, lambda root, widgets: interact(root, widgets, saving=True)))
                self.assertEqual(config["audio_cue"], sound)
                self.assertEqual(config["notify_mode"], "all" if visual else "none")
                self.assertFalse(self.run_dialog(services, lambda root, widgets: interact(root, widgets, saving=False)))

    def test_preview_failure_is_visible_without_key_or_saving(self):
        save = mock.Mock()
        services = SettingsServices(
            load_config=lambda: build_setup_config({"audio_cue": False, "notify_mode": "none"}),
            load_key=lambda: "", save=save,
            diagnostics=lambda: ("OK", True), lock=nullcontext,
            preview_audio=mock.Mock(side_effect=RuntimeError("Audio output unavailable")),
        )

        def interact(root, controls):
            next(w for w in controls if w.winfo_class() == "TButton" and w.cget("text") == "Test sound").invoke()
            messages = [root.getvar(w.cget("textvariable")) for w in controls
                        if w.winfo_class() == "TLabel" and w.cget("textvariable")]
            self.assertIn("Audio output unavailable", messages)
            next(w for w in controls if w.winfo_class() == "TButton" and w.cget("text") == "Cancel").invoke()

        self.assertFalse(self.run_dialog(services, interact))
        save.assert_not_called()
