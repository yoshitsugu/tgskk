// SPDX-License-Identifier: GPL-3.0-or-later
// uuid.lib does not supply the InputScope property GUID on all toolchains.
#include <windows.h>
#include <initguid.h>
#include <inputscope.h>
#ifdef _MSC_VER
DEFINE_GUID(GUID_PROP_INPUTSCOPE, 0x1713dd5a, 0x68e7, 0x4a5b, 0x9a, 0xf6,
            0x59, 0x2a, 0x59, 0x5c, 0x77, 0x8d);
#endif
