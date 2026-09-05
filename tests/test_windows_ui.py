"""Exercise real Tk widget construction and save/cancel on Windows CI."""
import os
import runpy
import tempfile
import unittest
from contextlib import nullcontext
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


@unittest.skipUnless(os.name == "nt", "Windows Tk dialog")
class WindowsDialogTests(unittest.TestCase):
    def test_save_and_cancel(self):
        import tkinter as tk
        from ai_dikte_windows import show_dialog

        namespace = runpy.run_path(str(Path(__file__).parents[1] / "ai-dikte"))
        for action in ("Save", "Cancel"):
            with self.subTest(action=action), tempfile.TemporaryDirectory() as directory:
                save = mock.Mock()
                startup = mock.Mock()
                runtime = SimpleNamespace(**namespace)
                runtime.FileLock = lambda path: nullcontext()
                runtime.load_config = lambda required=False: {}
                runtime.load_api_key = lambda required=False: "test-key"
                runtime.list_input_devices = lambda: [(None, "System default")]
                runtime.windows_startup_enabled = lambda: False
                runtime.save_settings = save
                runtime.set_windows_startup_enabled = startup
                runtime.LOCK_FILE = Path(directory) / "lock"
                root = tk.Tk()
                callback_errors = []

                def descendants(widget):
                    for child in widget.winfo_children():
                        yield child
                        yield from descendants(child)

                def interact():
                    try:
                        button = next(w for w in descendants(root)
                                      if w.winfo_class() == "TButton" and w.cget("text") == action)
                        button.invoke()
                    except Exception as exc:
                        callback_errors.append(exc)
                        root.destroy()

                root.after(100, interact)
                root.after(5000, root.destroy)
                with mock.patch("tkinter.Tk", return_value=root):
                    result = show_dialog("setup", runtime)
                self.assertEqual(callback_errors, [])
                self.assertEqual(result, action == "Save")
                if action == "Save":
                    save.assert_called_once()
                    self.assertEqual(save.call_args.args[0], "test-key")
                    self.assertEqual(save.call_args.args[1]["language"], "tr-TR")
                    self.assertNotIn("ui_language", save.call_args.args[1])
                    startup.assert_called_once()
                else:
                    save.assert_not_called()
                    startup.assert_not_called()
