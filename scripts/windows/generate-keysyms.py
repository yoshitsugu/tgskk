#!/usr/bin/env python3
import re
from pathlib import Path
header = Path('/usr/include/xkbcommon/xkbcommon-keysyms.h').read_text()
entries = re.findall(r'^#define XKB_KEY_(\w+)\s+(0x[0-9a-fA-F]+)\b', header, re.M)
license = header[header.index('/***********************************************************'):header.index('#define XKB_KEY_VoidSymbol')]
output = '// Generated from libxkbcommon xkbcommon-keysyms.h; regenerate with scripts/windows/generate-keysyms.py.\n' + license + '\n#[allow(dead_code, non_upper_case_globals)]\npub mod keysyms {\n'
output += ''.join(f'    pub const KEY_{name}: u32 = {value};\n' for name, value in entries)
output += '}\npub const NAMES: &[(&str, u32)] = &[\n' + ''.join(f'    ("{name}", {value}),\n' for name, value in sorted(entries)) + '];\n'
Path('third_party/cskk/cskk/src/portable_keysyms_data.rs').write_text(output)
