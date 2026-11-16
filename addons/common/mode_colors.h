// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <string_view>

namespace tgskk {
struct ModeColors {
  uint32_t background, foreground, border;
};

inline ModeColors modeColors(std::string_view label) {
  if (label == "ア")
    return {0x333C29, 0xF3F0E4, 0xB8A779};
  if (label == "ｱ")
    return {0x465E73, 0xF3F0E4, 0xB8A779};
  if (label == "A")
    return {0xB8A779, 0x292D23, 0x4F5838};
  if (label == "Ａ")
    return {0x5F3315, 0xF3F0E4, 0x333C29};
  return {0x4F5838, 0xF3F0E4, 0xB8A779};
}
} // namespace tgskk
