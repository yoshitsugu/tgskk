// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "../common/candidates.h"
#include "broker_platform.h"
#include "tgskk.h"
#include <array>
#include <stdexcept>

namespace tgskk::windows {
inline constexpr uint32_t wireMagic = 0x31424754, wireVersion = 1;
inline constexpr uint64_t keyBudget = 50, helloBudget = 3000;
inline constexpr uint32_t reloadDeferred = 32;
inline constexpr uint8_t asciiMode = 4;
enum class Op : uint32_t {
  Hello,
  Key,
  Choose,
  Select,
  Reset,
  Mode,
  Flush,
  Status
};
inline uint8_t modeFromLabel(const char *label) {
  const std::string value = label ? label : "";
  return value == "ア"   ? 1
         : value == "ｱ"  ? 2
         : value == "Ａ" ? 3
         : value == "A"  ? asciiMode
                         : 0;
}
struct Request {
  Op op = Op::Hello;
  uint64_t sequence = 1, deadline = 0, fingerprint = 0, generation = 0;
  uint32_t argument = 0, modifiers = 0, permissions = 0;
  std::string context;
};
struct Candidate {
  std::string text, annotation;
};
struct Snapshot {
  uint64_t sequence = 0, epoch = 0, generation = 0, processedKeys = 0;
  uint32_t flags = 0, selected = 0, count = 0, start = 0, mode = 0;
  uint32_t ai = 3, persistence = 0;
  std::string committed, preedit, label, error;
  std::array<uint8_t, 32> eligibility{};
  std::vector<Candidate> candidates;
  bool failed = false;
  bool eligible(uint32_t symbol, uint32_t modifiers) const;
};
inline constexpr std::array<uint32_t, 10> specialKeys = {
    TGSKK_KEY_BACKSPACE, TGSKK_KEY_TAB,      TGSKK_KEY_RETURN, TGSKK_KEY_ESCAPE,
    TGSKK_KEY_LEFT,      TGSKK_KEY_UP,       TGSKK_KEY_RIGHT,  TGSKK_KEY_DOWN,
    TGSKK_KEY_PAGE_UP,   TGSKK_KEY_PAGE_DOWN};
inline int eligibilityIndex(uint32_t symbol, uint32_t modifiers) {
  if (modifiers & ~(TGSKK_MOD_SHIFT | TGSKK_MOD_CONTROL))
    return -1;
  if (symbol >= 0x20 && symbol <= 0x7e) {
    if (!(modifiers & TGSKK_MOD_CONTROL))
      return -2;
    return static_cast<int>((modifiers & TGSKK_MOD_SHIFT ? 95 : 0) + symbol -
                            0x20);
  }
  for (size_t i = 0; i < specialKeys.size(); ++i)
    if (symbol == specialKeys[i])
      return static_cast<int>(190 + i * 4 +
                              ((modifiers & TGSKK_MOD_CONTROL) ? 2 : 0) +
                              ((modifiers & TGSKK_MOD_SHIFT) ? 1 : 0));
  return -1;
}
inline bool isAsciiPassthrough(uint32_t mode, uint32_t symbol,
                               uint32_t modifiers) {
  return mode == asciiMode && eligibilityIndex(symbol, modifiers) == -2;
}
inline bool Snapshot::eligible(uint32_t symbol, uint32_t modifiers) const {
  if (isAsciiPassthrough(mode, symbol, modifiers))
    return false;
  const int i = eligibilityIndex(symbol, modifiers);
  return i == -2 ||
         (i >= 0 && (eligibility[static_cast<size_t>(i) / 8] & (1 << (i % 8))));
}
class Writer {
public:
  std::vector<uint8_t> data;
  void u32(uint32_t n) {
    for (unsigned i = 0; i < 4; ++i)
      data.push_back(static_cast<uint8_t>(n >> (i * 8)));
  }
  void u64(uint64_t n) {
    u32(static_cast<uint32_t>(n));
    u32(static_cast<uint32_t>(n >> 32));
  }
  void string(const std::string &s) {
    if (s.size() > 8192)
      throw std::runtime_error("Protocol string limit");
    wide(s);
    u32(static_cast<uint32_t>(s.size()));
    data.insert(data.end(), s.begin(), s.end());
    if (data.size() > 65536)
      throw std::runtime_error("Protocol frame limit");
  }
};
class Reader {
  const std::vector<uint8_t> &data;
  size_t offset = 0;

public:
  explicit Reader(const std::vector<uint8_t> &bytes) : data(bytes) {
    if (data.size() > 65536)
      throw std::runtime_error("Protocol frame limit");
  }
  uint32_t u32() {
    if (data.size() - offset < 4)
      throw std::runtime_error("Truncated protocol integer");
    uint32_t n = 0;
    for (unsigned i = 0; i < 4; ++i)
      n |= uint32_t(data[offset++]) << (i * 8);
    return n;
  }
  uint64_t u64() {
    const uint64_t low = u32();
    return low | (uint64_t(u32()) << 32);
  }
  std::string string() {
    const auto n = u32();
    if (n > 8192 || n > data.size() - offset)
      throw std::runtime_error("Protocol string limit");
    std::string s(data.begin() + offset, data.begin() + offset + n);
    offset += n;
    wide(s);
    return s;
  }
  void end() {
    if (offset != data.size())
      throw std::runtime_error("Trailing protocol data");
  }
  void header() {
    if (u32() != wireMagic || u32() != wireVersion)
      throw std::runtime_error("Broker protocol version");
  }
};
inline std::vector<uint8_t> encode(const Request &q) {
  Writer w;
  w.u32(wireMagic);
  w.u32(wireVersion);
  w.u32(static_cast<uint32_t>(q.op));
  w.u64(q.sequence);
  w.u64(q.deadline);
  w.u64(q.fingerprint);
  w.u64(q.generation);
  w.u32(q.argument);
  w.u32(q.modifiers);
  w.u32(q.permissions);
  w.string(q.context);
  return std::move(w.data);
}
inline Request request(const std::vector<uint8_t> &bytes) {
  Reader r(bytes);
  r.header();
  Request q;
  const auto op = r.u32();
  if (op > static_cast<uint32_t>(Op::Status))
    throw std::runtime_error("Unknown broker operation");
  q.op = static_cast<Op>(op);
  q.sequence = r.u64();
  q.deadline = r.u64();
  q.fingerprint = r.u64();
  q.generation = r.u64();
  q.argument = r.u32();
  q.modifiers = r.u32();
  q.permissions = r.u32();
  q.context = r.string();
  r.end();
  if ((q.permissions & ~7U) || q.context.size() > 4096 ||
      (!(q.permissions & 1) && !q.context.empty()))
    throw std::runtime_error("Invalid request permissions");
  return q;
}
inline std::vector<uint8_t> encode(const Snapshot &s) {
  Writer w;
  w.u32(wireMagic);
  w.u32(wireVersion);
  w.u64(s.sequence);
  w.u64(s.epoch);
  w.u64(s.generation);
  w.u64(s.processedKeys);
  w.u32(s.failed);
  w.u32(s.flags);
  w.u32(s.selected);
  w.u32(s.count);
  w.u32(s.start);
  w.u32(s.mode);
  w.u32(s.ai);
  w.u32(s.persistence);
  w.string(s.committed);
  w.string(s.preedit);
  w.string(s.label);
  w.string(s.error);
  for (size_t i = 0; i < 32; i += 4) {
    uint32_t n = 0;
    for (size_t j = 0; j < 4; ++j)
      n |= uint32_t(s.eligibility[i + j]) << (j * 8);
    w.u32(n);
  }
  w.u32(static_cast<uint32_t>(s.candidates.size()));
  for (const auto &c : s.candidates) {
    w.string(c.text);
    w.string(c.annotation);
  }
  return std::move(w.data);
}
inline Snapshot snapshot(const std::vector<uint8_t> &bytes,
                         uint32_t inlineCandidates = 4) {
  Reader r(bytes);
  r.header();
  Snapshot s;
  s.sequence = r.u64();
  s.epoch = r.u64();
  s.generation = r.u64();
  s.processedKeys = r.u64();
  const auto failed = r.u32();
  if (failed > 1)
    throw std::runtime_error("Invalid reply status");
  s.failed = failed != 0;
  s.flags = r.u32();
  s.selected = r.u32();
  s.count = r.u32();
  s.start = r.u32();
  s.mode = r.u32();
  s.ai = r.u32();
  s.persistence = r.u32();
  s.committed = r.string();
  s.preedit = r.string();
  s.label = r.string();
  s.error = r.string();
  for (size_t i = 0; i < 32; i += 4) {
    const auto n = r.u32();
    for (size_t j = 0; j < 4; ++j)
      s.eligibility[i + j] = static_cast<uint8_t>(n >> (j * 8));
  }
  const auto count = r.u32();
  if (count > 7)
    throw std::runtime_error("Candidate page limit");
  for (uint32_t i = 0; i < count; ++i)
    s.candidates.push_back({r.string(), r.string()});
  r.end();
  if (!s.epoch || (s.flags & ~63U) || s.mode > 4 || s.ai > 3 ||
      s.persistence > 3 || s.count > 1000000 ||
      (s.count && s.selected >= s.count) || s.start > s.count ||
      count != std::min<uint32_t>(7, s.count - s.start) ||
      ((s.flags & 4) &&
       (!s.count ||
        s.start != tgskk::candidatePageStart(s.selected, inlineCandidates))))
    throw std::runtime_error("Invalid broker view");
  if ((s.flags & reloadDeferred) &&
      (!s.failed || s.count || !s.preedit.empty() || !s.committed.empty()))
    throw std::runtime_error("Invalid deferred broker request");
  return s;
}
} // namespace tgskk::windows
