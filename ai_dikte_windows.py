"""Small English-only Windows settings and diagnostics dialogs.

The runtime is supplied by the entrypoint to share configuration and validation
without importing or executing a second copy of the dictation engine.
"""
from __future__ import annotations

import queue
import threading
from pathlib import Path
from typing import Any


def show_dialog(command: str, runtime: Any) -> bool:
    import tkinter as tk
    from tkinter import messagebox, ttk

    root = tk.Tk()
    root.title("AI Dikte — Settings" if command == "setup" else "AI Dikte — Diagnostics")
    root.resizable(True, True)
    frame = ttk.Frame(root, padding=20)
    frame.pack(fill="both", expand=True)
    frame.columnconfigure(1, weight=1)
    saved = False
    busy = False
    results: queue.Queue[tuple[bool, str]] = queue.Queue()

    try:
        icon = Path(__file__).with_name("ai-dikte.png")
        if icon.is_file():
            root.iconphoto(True, tk.PhotoImage(file=str(icon)))

        if command == "doctor":
            report, ok = runtime.doctor_report()
            ttk.Label(frame, text="All checks passed" if ok else "Attention required").pack(anchor="w")
            from tkinter.scrolledtext import ScrolledText
            output = ScrolledText(frame, width=76, height=20, wrap="word")
            output.pack(fill="both", expand=True, pady=12)
            output.insert("1.0", report)
            output.configure(state="disabled")
            ttk.Button(frame, text="Close", command=root.destroy).pack(anchor="e")
            root.mainloop()
            return ok
        if command != "setup":
            raise RuntimeError(f"Unknown dialog: {command}")

        with runtime.FileLock(runtime.LOCK_FILE):
            existing = runtime.build_setup_config(runtime.load_config(required=False))
            old_key = runtime.load_api_key(required=False)
            key = tk.StringVar(value=old_key)
            language = tk.StringVar(value=runtime.config_language(existing))
            mode = tk.StringVar(value=runtime.config_mode(existing))
            cue = tk.BooleanVar(value=runtime.config_audio_cue(existing))
            notifications = tk.BooleanVar(value=runtime.config_notify_mode(existing) == "all")
            startup = tk.BooleanVar(value=runtime.windows_startup_enabled() if old_key else True)
            status = tk.StringVar(value="API changes are checked online. Local preferences can be saved offline.")

            ttk.Label(frame, text="AI Dikte", font=("Segoe UI", 18, "bold")).grid(
                row=0, column=0, columnspan=2, sticky="w", pady=(0, 14))
            ttk.Label(frame, text="Google AI API key").grid(row=1, column=0, sticky="w", padx=(0, 14))
            key_entry = ttk.Entry(frame, textvariable=key, show="*", width=45)
            key_entry.grid(row=1, column=1, sticky="ew", pady=5)
            show_key = tk.BooleanVar()
            ttk.Checkbutton(frame, text="Show key", variable=show_key,
                            command=lambda: key_entry.configure(show="" if show_key.get() else "*")
                            ).grid(row=2, column=1, sticky="w")

            ttk.Label(frame, text="Dictation language").grid(row=3, column=0, sticky="w")
            ttk.Combobox(frame, textvariable=language, values=("tr-TR", "en-US"), width=24).grid(
                row=3, column=1, sticky="ew", pady=(12, 3))
            ttk.Label(frame, text="tr-TR: Turkish · en-US: English · or enter another language code").grid(
                row=4, column=0, columnspan=2, sticky="w", pady=(0, 10))
            ttk.Label(frame, text="Transcription mode").grid(row=5, column=0, sticky="w")
            ttk.Combobox(frame, textvariable=mode, values=("SMART", "VERBATIM"), state="readonly").grid(
                row=5, column=1, sticky="ew", pady=5)
            ttk.Label(frame, text="SMART cleans up speech; VERBATIM preserves spoken repetitions.").grid(
                row=6, column=0, columnspan=2, sticky="w", pady=(0, 10))

            try:
                devices = runtime.list_input_devices()
            except Exception as exc:
                devices = [(None, "System default")]
                status.set(f"Could not list microphones: {exc}")
            selected = runtime.config_input_device(existing)
            if selected not in [index for index, _ in devices]:
                devices.append((selected, f"{selected}: Unavailable"))
            device_labels = [label for _, label in devices]
            device = tk.StringVar(value=next(label for index, label in devices if index == selected))
            ttk.Label(frame, text="Microphone").grid(row=7, column=0, sticky="w")
            ttk.Combobox(frame, textvariable=device, values=device_labels, state="readonly").grid(
                row=7, column=1, sticky="ew", pady=5)
            ttk.Checkbutton(frame, text="Play recording sounds", variable=cue).grid(
                row=8, column=0, columnspan=2, sticky="w", pady=(10, 0))
            ttk.Checkbutton(frame, text="Show notifications (errors always appear)", variable=notifications).grid(
                row=9, column=0, columnspan=2, sticky="w")
            ttk.Checkbutton(frame, text="Start with Windows", variable=startup).grid(
                row=10, column=0, columnspan=2, sticky="w")
            ttk.Label(frame, text="Custom words — one per line").grid(
                row=11, column=0, columnspan=2, sticky="w", pady=(14, 5))
            from tkinter.scrolledtext import ScrolledText
            vocabulary = ScrolledText(frame, width=55, height=5, wrap="word")
            vocabulary.grid(row=12, column=0, columnspan=2, sticky="nsew")
            frame.rowconfigure(12, weight=1)
            vocabulary.insert("1.0", "\n".join(runtime.config_vocabulary(existing)))
            ttk.Label(frame, textvariable=status, wraplength=520).grid(
                row=13, column=0, columnspan=2, sticky="w", pady=12)
            buttons = ttk.Frame(frame)
            buttons.grid(row=14, column=0, columnspan=2, sticky="e")

            def close() -> None:
                if not busy:
                    root.destroy()

            def worker(candidate: dict, candidate_key: str, start_enabled: bool) -> None:
                try:
                    runtime.save_settings(candidate_key, candidate)
                except Exception as exc:
                    results.put((False, str(exc)))
                    return
                try:
                    runtime.set_windows_startup_enabled(start_enabled)
                except Exception as exc:
                    results.put((True, f"Dictation settings saved, but startup could not be changed: {exc}"))
                    return
                results.put((True, ""))

            def poll() -> None:
                nonlocal busy, saved
                try:
                    success, error = results.get_nowait()
                except queue.Empty:
                    root.after(100, poll)
                    return
                busy = False
                if success:
                    saved = True
                    if error:
                        messagebox.showwarning("AI Dikte", error, parent=root)
                    root.destroy()
                else:
                    status.set(error)
                    save_button.configure(state="normal")
                    cancel_button.configure(state="normal")

            def save(_event: Any = None) -> None:
                nonlocal busy
                if busy:
                    return
                try:
                    candidate = runtime.build_setup_config(existing, {
                        "language": language.get().strip(),
                        "mode": mode.get(),
                        "input_device": devices[device_labels.index(device.get())][0],
                        "audio_cue": cue.get(),
                        "notify_mode": "all" if notifications.get() else "none",
                        "custom_vocabulary": vocabulary.get("1.0", "end").splitlines(),
                    })
                    candidate_key = key.get().strip()
                    if not candidate_key:
                        raise RuntimeError("API key cannot be empty.")
                except Exception as exc:
                    status.set(str(exc))
                    return
                busy = True
                status.set("Saving settings… API changes may take a few seconds to validate.")
                save_button.configure(state="disabled")
                cancel_button.configure(state="disabled")
                threading.Thread(target=worker, args=(candidate, candidate_key, startup.get()), daemon=True).start()
                root.after(100, poll)

            cancel_button = ttk.Button(buttons, text="Cancel", command=close)
            cancel_button.pack(side="left", padx=6)
            save_button = ttk.Button(buttons, text="Save", command=save)
            save_button.pack(side="left")
            root.protocol("WM_DELETE_WINDOW", close)
            root.bind("<Escape>", lambda _event: close())
            root.bind("<Control-Return>", save)
            root.update_idletasks()
            root.minsize(root.winfo_reqwidth(), root.winfo_reqheight())
            key_entry.focus_set()
            root.mainloop()
        return saved
    finally:
        try:
            root.destroy()
        except tk.TclError:
            pass
