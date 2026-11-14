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
    std::puts("IME mode mapping, invalid flags and unrelated OS flags passed");
    return 0;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }
}
