"""Real widget smoke tests run on Windows and under Xvfb in Linux CI."""
import os
import unittest
from contextlib import nullcontext
from unittest import mock
from ai_dikte_config import build_setup_config
from ai_dikte_ui import SettingsServices, show_dialog


@unittest.skipUnless(os.name == "nt" or os.environ.get("DISPLAY"), "GUI display required")
class DialogTests(unittest.TestCase):
    def test_save_cancel_and_platform_controls(self):
        import tkinter as tk
        for windows_controls in (False, True):
            for action in ("Save", "Cancel"):
                with self.subTest(windows=windows_controls, action=action):
                    save, startup = mock.Mock(), mock.Mock()
                    services = SettingsServices(
                        load_config=lambda: build_setup_config({}),
                        load_key=lambda: "test-key", save=save,
                        diagnostics=lambda: ("OK", True), lock=nullcontext,
                        devices=(lambda: [(None, "System default")]) if windows_controls else None,
                        get_startup=(lambda: False) if windows_controls else None,
                        set_startup=startup if windows_controls else None,
                    )
                    root = tk.Tk()
                    errors = []
                    def descendants(widget):
                        for child in widget.winfo_children():
                            yield child
                            yield from descendants(child)
                    def interact():
                        try:
                            controls = list(descendants(root))
                            startup_widgets = [w for w in controls if "text" in w.keys() and w.cget("text") == "Start with Windows"]
                            self.assertEqual(bool(startup_widgets), windows_controls)
                            next(w for w in controls if w.winfo_class() == "TButton" and w.cget("text") == action).invoke()
                        except Exception as exc:
                            errors.append(exc)
                            root.destroy()
                    root.after(100, interact)
                    timer = root.after(5000, root.destroy)
                    with mock.patch("tkinter.Tk", return_value=root):
                        result = show_dialog("setup", services)
                    self.assertEqual(errors, [])
                    self.assertEqual(result, action == "Save")
                    if action == "Save":
                        save.assert_called_once()
                        self.assertEqual(save.call_args.args[1]["language"], "tr-TR")
                        if windows_controls:
                            startup.assert_called_once_with(False)
                        else:
                            self.assertIsNone(save.call_args.args[1]["input_device"])
                            startup.assert_not_called()
                    else:
                        save.assert_not_called()
                        startup.assert_not_called()
