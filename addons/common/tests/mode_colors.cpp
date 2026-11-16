// SPDX-License-Identifier: GPL-3.0-or-later
#include "../mode_colors.h"
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string_view>
using tgskk::modeColors;
static void require(bool value, const char *reason) {
  if (!value) {
    std::fprintf(stderr, "%s\n", reason);
    std::exit(1);
  }
}
static void colors(std::string_view label, uint32_t background,
                   uint32_t foreground, uint32_t border) {
  const auto result = modeColors(label);
  require(result.background == background && result.foreground == foreground &&
              result.border == border,
          "wrong mode colors");
}
int main() {
  colors("あ", 0x4F5838, 0xF3F0E4, 0xB8A779);
  colors("ア", 0x333C29, 0xF3F0E4, 0xB8A779);
  colors("ｱ", 0x465E73, 0xF3F0E4, 0xB8A779);
  colors("A", 0xB8A779, 0x292D23, 0x4F5838);
  colors("Ａ", 0x5F3315, 0xF3F0E4, 0x333C29);
  colors("", 0x4F5838, 0xF3F0E4, 0xB8A779);
  colors("unknown", 0x4F5838, 0xF3F0E4, 0xB8A779);
}
