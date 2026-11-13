// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <algorithm>
#include <cstddef>

namespace tgskk {
inline constexpr std::size_t candidatePageSize = 7;
inline constexpr std::size_t defaultInlineCandidates = 4;

enum class CandidateMoveKind { Stay, Choose, Register, ReturnToReading };
struct CandidateMove {
  CandidateMoveKind kind;
  std::size_t index;
};

inline std::size_t
candidatePageStart(std::size_t selected, std::size_t inlineCount,
                   std::size_t pageSize = candidatePageSize) {
  if (selected < inlineCount || !pageSize)
    return 0;
  return inlineCount + (selected - inlineCount) / pageSize * pageSize;
}

inline CandidateMove
candidatePageMove(std::size_t count, std::size_t selected,
                  std::size_t inlineCount, bool forward,
                  bool registerAtEnd = false,
                  std::size_t pageSize = candidatePageSize) {
  if (!count || selected >= count || !pageSize)
    return {CandidateMoveKind::Stay, selected};
  if (selected < inlineCount) {
    return forward ? CandidateMove{CandidateMoveKind::Choose,
                                   std::min(inlineCount, count - 1)}
                   : CandidateMove{CandidateMoveKind::Stay, selected};
  }
  const auto start = candidatePageStart(selected, inlineCount, pageSize);
  if (forward) {
    if (count - start <= pageSize)
      return registerAtEnd
                 ? CandidateMove{CandidateMoveKind::Register, count - 1}
                 : CandidateMove{CandidateMoveKind::Stay, selected};
    return {CandidateMoveKind::Choose, start + pageSize};
  }
  if (start > inlineCount)
    return {CandidateMoveKind::Choose, start - pageSize};
  if (inlineCount)
    return {CandidateMoveKind::Choose, inlineCount - 1};
  return {CandidateMoveKind::ReturnToReading, 0};
}

inline std::size_t candidateStep(std::size_t count, std::size_t selected,
                                 bool forward) {
  if (!count || selected >= count)
    return 0;
  return forward ? std::min(selected + 1, count - 1)
                 : (selected ? selected - 1 : 0);
}
} // namespace tgskk
