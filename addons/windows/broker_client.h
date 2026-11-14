// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "broker_protocol.h"
#include <atomic>
#include <mutex>
#include <utility>

namespace tgskk::windows {
class Connection {
  Channel channel;
  uint64_t sequence = 0, fingerprint;
  bool failed = false;

public:
  Snapshot current;
  Connection(Channel, uint64_t fingerprint);
  static std::unique_ptr<Connection> connect(const Config &);
  const Snapshot &perform(Op, uint32_t argument = 0, uint32_t modifiers = 0,
                          uint32_t permissions = 0, std::string context = {},
                          uint64_t budget = keyBudget);
  HANDLE handle() const { return channel.handle(); }
};
struct Connecting {
  std::atomic<bool> stopped = false, finished = false;
  std::mutex mutex;
  std::unique_ptr<Connection> ready;
  std::string error;
};
std::shared_ptr<Connecting> connectAsync(const Config &, HMODULE,
                                         std::atomic<long> *);
class Input {
  std::unique_ptr<TgskkRuntime, decltype(&tgskk_runtime_free)> runtime{
      nullptr, tgskk_runtime_free};
  std::unique_ptr<TgskkSession, decltype(&tgskk_session_free)> local{
      nullptr, tgskk_session_free};
  std::unique_ptr<Connection> remote;
  std::shared_ptr<Connecting> connecting;
  Config config;
  HMODULE module;
  std::atomic<long> *objects;
  TgskkView cached{};
  std::string diagnostic, lastRemoteError;
  bool transportFailed = false;
  uint64_t retryAt = 0, retryDelay = 1000;
  void startConnecting();
  void adopt();
  const TgskkView *action(Op, uint32_t = 0, uint32_t = 0);
  const TgskkView *remoteView();
  void fail(const std::string &);

public:
  Input(const Config &, HMODULE, std::atomic<long> *);
  ~Input();
  Input(const Input &) = delete;
  const TgskkView *view();
  const TgskkView *key(uint32_t symbol, uint32_t modifiers);
  const TgskkView *choose(size_t index);
  const TgskkView *select(size_t index);
  const TgskkView *reset();
  bool mayProcess(uint32_t symbol, uint32_t modifiers);
  const char *candidate(size_t index, bool annotation = false);
  void context(std::string text, bool ai, bool network);
  const char *error() const { return diagnostic.c_str(); }
  std::string takeDiagnostic() { return std::exchange(diagnostic, {}); }
  bool takeTransportFailure() { return std::exchange(transportFailed, false); }
  bool broker() const { return bool(remote); }

private:
  std::string left;
  uint32_t permissions = 0;
};
} // namespace tgskk::windows
