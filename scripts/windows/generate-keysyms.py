#!/usr/bin/env python3
import argparse
import hashlib
import re
from pathlib import Path
parser = argparse.ArgumentParser(description="Regenerate the pinned Windows keysym table")
parser.add_argument("--header", type=Path, default=Path("/usr/include/xkbcommon/xkbcommon-keysyms.h"))
args = parser.parse_args()
source = args.header.read_bytes()
expected = "13023369f65a17411606084e3e09557b4886aeb15f89affba4aaa86490a463f3"
if hashlib.sha256(source).hexdigest() != expected:
    parser.error("Expected the libxkbcommon 1.13.2 header; review the pin before upgrading")
header = source.decode("utf-8")
entries = re.findall(r'^#define XKB_KEY_(\w+)\s+(0x[0-9a-fA-F]+)\b', header, re.M)
license = header[header.index('/***********************************************************'):header.index('#define XKB_KEY_VoidSymbol')]
output = '// Generated from libxkbcommon xkbcommon-keysyms.h; regenerate with scripts/windows/generate-keysyms.py.\n' + license + '\n#[allow(dead_code, non_upper_case_globals)]\npub mod keysyms {\n'
output += ''.join(f'    pub const KEY_{name}: u32 = {value};\n' for name, value in entries)
output += '}\npub const NAMES: &[(&str, u32)] = &[\n' + ''.join(f'    ("{name}", {value}),\n' for name, value in sorted(entries)) + '];\n'
root = Path(__file__).resolve().parents[2]
(root / 'third_party/cskk/cskk/src/portable_keysyms_data.rs').write_text(output, encoding='utf-8')
