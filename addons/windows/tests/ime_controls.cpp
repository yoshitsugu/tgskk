// SPDX-License-Identifier: GPL-3.0-or-later
#include "../ime_controls.h"
#include <cstdio>
#include <stdexcept>

static void require(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}
int main() {
  try {
    bool keyboard = false, inputMode = false, tray = false;
    for (const auto &category : registeredCategories) {
      keyboard |= category == GUID_TFCAT_TIP_KEYBOARD;
      inputMode |= category == GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT;
      tray |= category == systemTrayCategory;
    }
    require(keyboard && inputMode && tray && registeredCategories.size() == 3,
            "registered categories do not match implemented capabilities");
    for (uint8_t mode = 0; mode < imeModes.size(); ++mode) {
      const auto extra = IME_CMODE_ROMAN | IME_CMODE_SOFTKBD | IME_CMODE_SYMBOL;
      const auto flags = conversionFlags(mode, extra | imeModeMask);
      require((flags & ~imeModeMask) == extra,
              "discarded unrelated OS mode flags");
      require(modeFromFlags(flags, 4) == mode, "mode did not round-trip");
      require(modeFromFlags(IME_CMODE_KATAKANA, mode) == mode,
              "invalid mode changed input state");
    }
    require(modeFromFlags(IME_CMODE_NATIVE, 4) == 0,
            "native mode did not select hiragana");
    require(inputModeIconColor(false, false, RGB(1, 2, 3)) ==
                RGB(255, 255, 255),
            "dark taskbar did not use a light mode icon");
    require(inputModeIconColor(true, false, RGB(1, 2, 3)) == RGB(0, 0, 0),
            "light taskbar did not use a dark mode icon");
    require(inputModeIconColor(false, true, RGB(1, 2, 3)) == RGB(1, 2, 3),
            "high contrast did not use the system text color");
    std::puts("IME mode mapping, invalid flags, unrelated OS flags and icon "
              "colors passed");
    return 0;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }
}
