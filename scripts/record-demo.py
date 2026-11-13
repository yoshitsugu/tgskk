#!/usr/bin/env python3
import ctypes as C
import ctypes.util
import argparse
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess as S
import sys
import tempfile
import time
ROOT = Path(__file__).resolve().parent.parent

def session(scratch):
    config = scratch / 'config'
    (config / 'fcitx5/conf').mkdir(parents=True)
    (config / 'fcitx5/profile').write_text('[Groups/0]\nName=Default\nDefault Layout=us\nDefaultIM=tgskk\n\n[Groups/0/Items/0]\nName=keyboard-us\nLayout=\n\n[Groups/0/Items/1]\nName=tgskk\nLayout=\n\n[GroupOrder]\n0=Default\n')
    (config / 'fcitx5/conf/tgskk.conf').write_text(f'Dictionary=/usr/share/skk/SKK-JISYO.L\nEncoding=euc-jp\nModeIndicator=True\nZenzEnabled=True\nZenzModel={ROOT}/build/models/zenz-v3.2-small-Q5_K_M.gguf\n')
    for schema, key, value in [('org.gnome.gedit.preferences.editor', 'use-default-font', 'false'), ('org.gnome.gedit.preferences.editor', 'editor-font', 'Noto Sans CJK JP 32'), ('org.gnome.gedit.preferences.editor', 'display-line-numbers', 'true'), ('org.gnome.gedit.preferences.editor', 'style-scheme-for-dark-theme-variant', 'oblivion'), ('org.gnome.gedit.preferences.editor', 'highlight-current-line', 'false'), ('org.gnome.gedit.preferences.ui', 'theme-variant', 'dark')]:
        S.run(['gsettings', 'set', schema, key, value], check=True)
    x = C.CDLL(ctypes.util.find_library('X11'))
    xt = C.CDLL(ctypes.util.find_library('Xtst'))
    x.XOpenDisplay.argtypes = [C.c_char_p]
    x.XOpenDisplay.restype = C.c_void_p
    display = x.XOpenDisplay(None)
    if not display:
        raise RuntimeError('Cannot open recording display')
    x.XDefaultRootWindow.argtypes = [C.c_void_p]
    x.XDefaultRootWindow.restype = C.c_ulong
    root = x.XDefaultRootWindow(display)
    x.XQueryTree.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_ulong), C.POINTER(C.POINTER(C.c_ulong)), C.POINTER(C.c_uint)]
    x.XFetchName.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(C.c_char_p)]
    x.XFree.argtypes = [C.c_void_p]
    x.XMoveResizeWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint]
    x.XSetInputFocus.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_ulong]
    x.XFlush.argtypes = [C.c_void_p]
    x.XStringToKeysym.argtypes = [C.c_char_p]
    x.XStringToKeysym.restype = C.c_ulong
    x.XKeysymToKeycode.argtypes = [C.c_void_p, C.c_ulong]
    x.XKeysymToKeycode.restype = C.c_ubyte
    xt.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
    xt.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
    xt.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]

    def windows():
        root_out = C.c_ulong()
        parent = C.c_ulong()
        children = C.POINTER(C.c_ulong)()
        count = C.c_uint()
        x.XQueryTree(display, root, C.byref(root_out), C.byref(parent), C.byref(children), C.byref(count))
        out = []
        for i in range(count.value):
            title = C.c_char_p()
            if x.XFetchName(display, children[i], C.byref(title)) and title.value:
                out.append((children[i], title.value.decode(errors='replace')))
                x.XFree(title)
        if children:
            x.XFree(children)
        return out

    def key(name, modifier=None):

        def event(name, down):
            code = x.XKeysymToKeycode(display, x.XStringToKeysym(name.encode()))
            if not code:
                raise ValueError(name)
            xt.XTestFakeKeyEvent(display, code, down, 0)
        if modifier:
            event(modifier, True)
        event(name, True)
        event(name, False)
        if modifier:
            event(modifier, False)
        x.XFlush(display)
    processes = []

    def start(args, log):
        handle = open(scratch / log, 'wb')
        p = S.Popen(args, stdout=handle, stderr=S.STDOUT)
        handle.close()
        processes.append(p)
        return p
    try:
        fcitx = start(['fcitx5', '-D', '--disable=all', '--enable=tgskk,keyboard,xcb,dbus,dbusfrontend,classicui,xim', '-u', 'classicui'], 'fcitx.log')
        editor = start(['gedit', '--standalone'], 'gedit.log')
        until = time.monotonic() + 10
        window = None
        while time.monotonic() < until:
            ws = windows()
            print(ws, flush=True)
            for wid, title in ws:
                if title != 'gedit' and ('gedit' in title.lower() or '無題' in title or 'Untitled' in title):
                    window = wid
                    break
            if window:
                break
            time.sleep(0.2)
        if not window:
            raise RuntimeError('gedit window not found')
        time.sleep(0.5)
        x.XMoveResizeWindow(display, window, 0, 0, 1120, 480)
        x.XSetInputFocus(display, window, 1, 0)
        xt.XTestFakeMotionEvent(display, 0, 180, 180, 0)
        xt.XTestFakeButtonEvent(display, 1, True, 0)
        xt.XTestFakeButtonEvent(display, 1, False, 0)
        x.XFlush(display)
        time.sleep(0.5)
        S.run(['fcitx5-remote', '-s', 'tgskk'], check=True)
        S.run(['fcitx5-remote', '-o'], check=True)
        time.sleep(1)
        print('input method:', S.check_output(['fcitx5-remote', '-n']).decode(), flush=True)
        recorder = start(['ffmpeg', '-y', '-v', 'warning', '-f', 'x11grab', '-draw_mouse', '0', '-framerate', '20', '-video_size', '1120x480', '-i', os.environ['DISPLAY'], '-c:v', 'libx264', '-threads', '2', '-crf', '18', '-pix_fmt', 'yuv420p', '-map_metadata', '-1', '-movflags', '+faststart', str(scratch / 'demo.mp4')], 'ffmpeg.log')
        time.sleep(1)
        key('j', 'Control_L')
        time.sleep(0.4)
        for reading in ['Eki', 'de', 'Kisha', '\n', 'Shinbun', 'no', 'Kisha']:
            if reading == '\n':
                key('Return')
                time.sleep(0.6)
                continue
            for ch in reading:
                key(ch.lower(), 'Shift_L' if ch.isupper() else None)
                time.sleep(0.18)
            if reading[0].isupper():
                key('space')
                time.sleep(1.2)
                key('Return')
                time.sleep(0.35)
        time.sleep(2)
        recorder.send_signal(signal.SIGINT)
        recorder.wait(timeout=8)
        if recorder.returncode not in (0, 255):
            raise RuntimeError('Recording failed')
        S.run(['ffmpeg', '-v', 'error', '-y', '-i', str(scratch / 'demo.mp4'), '-frames:v', '1', '-update', '1', str(scratch / 'first.png')], check=True)
        S.run(['ffmpeg', '-v', 'error', '-y', '-sseof', '-1', '-i', str(scratch / 'demo.mp4'), '-frames:v', '1', '-update', '1', str(scratch / 'last.png')], check=True)
    finally:
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
        for p in reversed(processes):
            try:
                p.wait(timeout=5)
            except S.TimeoutExpired:
                p.kill()
                p.wait()
if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--session':
        session(Path(sys.argv[2]))
        sys.exit()
    parser = argparse.ArgumentParser(description='Record real gedit input with fcitx5/tgskk on a private X11 display.')
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'docs/media')
    args = parser.parse_args()
    for required in [ROOT / 'build/fcitx5-zenz-test/fcitx5-tgskk.so', ROOT / 'build/models/zenz-v3.2-small-Q5_K_M.gguf', Path('/usr/share/skk/SKK-JISYO.L')]:
        if not required.is_file():
            parser.error(f'Missing {required}; see docs/media/README.md')
    scratch = Path(tempfile.mkdtemp(prefix='tgskk-real-demo-'))
    print('SCRATCH', scratch, flush=True)
    read, write = os.pipe()
    xvfb = S.Popen(['Xvfb', '-displayfd', str(write), '-screen', '0', '1120x480x24', '-nolisten', 'tcp', '-ac'], pass_fds=[write], stdout=S.DEVNULL, stderr=open(scratch / 'xvfb.log', 'wb'))
    os.close(write)
    try:
        if not select.select([read], [], [], 5)[0]:
            raise RuntimeError('Xvfb startup timed out')
        display = ':' + os.read(read, 100).decode().strip()
        os.close(read)
        env = os.environ.copy()
        for key in ['DBUS_SESSION_BUS_ADDRESS', 'WAYLAND_DISPLAY']:
            env.pop(key, None)
        env.update(FCITX_CONFIG_HOME=str(scratch / 'config/fcitx5'), FCITX_DATA_HOME=str(scratch / 'data/fcitx5'), NO_AT_BRIDGE='1', GIO_USE_VFS='local', GTK_USE_PORTAL='0', DISPLAY=display, XAUTHORITY='', GDK_BACKEND='x11', GTK_IM_MODULE='fcitx', XMODIFIERS='@im=fcitx', GTK_THEME='Adwaita:dark', GSETTINGS_BACKEND='keyfile', LANG='ja_JP.UTF-8', SKIP_FCITX_PATH='1', FCITX_ADDON_DIRS=f'{ROOT}/build/fcitx5-zenz-test:/usr/lib/fcitx5', FCITX_DATA_DIRS=f'{ROOT}/build/fcitx5-zenz-test/test-data:/usr/share/fcitx5')
        for variable, folder in [('XDG_CONFIG_HOME', 'config'), ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache'), ('XDG_STATE_HOME', 'state'), ('XDG_RUNTIME_DIR', 'runtime')]:
            (scratch / folder).mkdir(mode=448, exist_ok=True)
            env[variable] = str(scratch / folder)
        S.run(['dbus-run-session', '--', '/usr/bin/python3', str(Path(__file__).resolve()), '--session', str(scratch)], env=env, check=True, timeout=60)
    finally:
        xvfb.terminate()
        xvfb.wait(timeout=5)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(scratch / 'demo.mp4', args.output_dir / 'demo.mp4')
    S.run(['ffmpeg', '-v', 'error', '-y', '-i', str(scratch / 'demo.mp4'), '-filter_complex_threads', '2', '-filter_complex', '[0:v]fps=16,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3', '-loop', '0', str(args.output_dir / 'demo.gif')], check=True)
    shutil.rmtree(scratch)
    print(args.output_dir / 'demo.gif')
