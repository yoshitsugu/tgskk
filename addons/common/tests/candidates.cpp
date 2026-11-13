// SPDX-License-Identifier: GPL-3.0-or-later
#include "../candidates.h"
#include <cstdio>
#include <cstdlib>
#include <limits>
using tgskk::CandidateMoveKind;
static void require(bool value, const char *reason) {
  if (!value) {
    std::fprintf(stderr, "%s\n", reason);
    std::exit(1);
  }
}
static void move(std::size_t count, std::size_t selected, std::size_t offset,
                 bool forward, bool registration, CandidateMoveKind kind,
                 std::size_t index) {
  const auto result =
      tgskk::candidatePageMove(count, selected, offset, forward, registration);
  require(result.kind == kind && result.index == index,
          "wrong candidate page move");
}
int main() {
  move(14, 0, 4, true, false, CandidateMoveKind::Choose, 4);
  move(14, 3, 4, false, false, CandidateMoveKind::Stay, 3);
  move(3, 0, 4, true, false, CandidateMoveKind::Choose, 2);
  move(14, 4, 4, true, false, CandidateMoveKind::Choose, 11);
  move(14, 10, 4, true, true, CandidateMoveKind::Choose, 11);
  move(14, 11, 4, true, false, CandidateMoveKind::Stay, 11);
  move(14, 11, 4, true, true, CandidateMoveKind::Register, 13);
  move(14, 13, 4, false, false, CandidateMoveKind::Choose, 4);
  move(14, 6, 4, false, false, CandidateMoveKind::Choose, 3);
  move(14, 0, 0, false, false, CandidateMoveKind::ReturnToReading, 0);
  move(14, 0, 0, true, false, CandidateMoveKind::Choose, 7);
  move(14, 7, 0, true, false, CandidateMoveKind::Stay, 7);
  move(14, 7, 0, true, true, CandidateMoveKind::Register, 13);
  move(0, 0, 4, true, true, CandidateMoveKind::Stay, 0);
  const auto largest = std::numeric_limits<std::size_t>::max();
  move(largest, largest - 1, 4, true, false, CandidateMoveKind::Stay,
       largest - 1);
  require(tgskk::candidatePageStart(13, 4) == 11, "wrong page start");
  require(tgskk::candidatePageStart(10, 4) == 4, "wrong first page start");
  require(tgskk::candidateStep(14, 13, true) == 13,
          "down crossed last candidate");
  require(tgskk::candidateStep(14, 0, false) == 0,
          "up crossed first candidate");
  require(tgskk::candidateStep(14, 4, false) == 3, "up did not return inline");
}
