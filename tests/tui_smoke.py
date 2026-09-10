"""Exercise the real Linux terminal backend without credentials or a desktop."""
import os
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
import fcntl
import pyte

binary = os.path.abspath(sys.argv[1])
with tempfile.TemporaryDirectory() as config:
    env = dict(os.environ, XDG_CONFIG_HOME=config, TERM="xterm-256color")
    result = subprocess.run([binary, "menu"], env=env, capture_output=True, timeout=5)
    assert result.returncode != 0 and b"interactive terminal" in result.stderr
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    original = termios.tcgetattr(slave)
    proc = subprocess.Popen([binary, "menu"], stdin=slave, stdout=slave, stderr=slave, env=env)
    output = bytearray()
    screen = pyte.Screen(80, 24)
    stream = pyte.ByteStream(screen)

    def wait_for(text):
        deadline = time.monotonic() + 5
        while text not in "\n".join(screen.display).encode():
            assert time.monotonic() < deadline, f"Missing terminal content: {text!r}; screen: {screen.display!r}"
            if select.select([master], [], [], 0.1)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                stream.feed(chunk)

    try:
        wait_for(b"Exit menu")
        assert not termios.tcgetattr(slave)[3] & termios.ICANON
        os.write(master, b"\x1b[B\r")  # Settings
        wait_for(b"Spoken language")
        os.write(master, b"\r")  # API key editor
        wait_for(b"Ctrl+U")
        os.write(master, b"fake-key-for-test")
        # A subsequent navigation barrier ensures the key events were consumed.
        os.write(master, b"\r")
        wait_for(b"(new key)")
        assert b"fake-key-for-test" not in output
        os.write(master, b"q")
        wait_for(b"Discard unsaved")
        os.write(master, b"y")
        assert proc.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == original, "Terminal mode was not restored"
        assert not os.path.exists(os.path.join(config, "ai-dikte", "config.json"))
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
print("TUI PTY smoke passed: navigation, secret masking, discard, terminal restoration, pipe rejection")
