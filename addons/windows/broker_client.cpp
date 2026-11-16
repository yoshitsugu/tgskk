// SPDX-License-Identifier: GPL-3.0-or-later
#include "broker_client.h"
#include <limits>
#include <thread>
#include <utility>

namespace tgskk::windows {
Connection::Connection(Channel value, uint64_t valueFingerprint,
                       uint32_t inlineCount)
    : channel(std::move(value)), fingerprint(valueFingerprint),
      inlineCandidates(inlineCount) {}
std::unique_ptr<Connection> Connection::connect(const Config &config) {
  const auto identity = Identity::current();
  const auto name = endpoint(identity, config.home, config.broker);
  auto handle = openPipe(name);
  if (!handle)
    return nullptr;
  verifyPeer(handle.value, false, identity, config.broker);
  Channel rendezvous(std::move(handle));
  std::vector<uint8_t> body;
  if (!rendezvous.read(body, monotonicMillis() + 100))
    return nullptr;
  Reader reader(body);
  const auto dedicated = wide(reader.string());
  reader.end();
  if (!dedicated.starts_with(name + L"-c-") || dedicated.size() > 256)
    throw std::runtime_error("Invalid dedicated pipe name");
  const auto delimiter = dedicated.rfind(L"-c-");
  size_t parsed = 0;
  const auto nonce = std::stoull(dedicated.substr(delimiter + 3), &parsed);
  if (parsed != dedicated.size() - delimiter - 3)
    throw std::runtime_error("Invalid pipe nonce");
  Writer ack;
  ack.u64(nonce);
  if (!rendezvous.write(ack.data, monotonicMillis() + 100))
    return nullptr;
  handle = openPipe(dedicated);
  if (!handle)
    return nullptr;
  verifyPeer(handle.value, false, identity, config.broker);
  auto connection = std::make_unique<Connection>(
      Channel(std::move(handle)), config.fingerprint, config.inlineCandidates);
  connection->perform(Op::Hello, 0, 0, 0, {}, {}, helloBudget);
  return connection;
}
const Snapshot &Connection::perform(Op op, uint32_t argument,
                                    uint32_t modifiers, uint32_t permissions,
                                    std::string context, std::string right,
                                    uint64_t budget) {
  if (failed)
    throw std::runtime_error("Discard failed broker connection");
  failed = true;
  Request q;
  q.op = op;
  q.sequence = ++sequence;
  q.deadline = monotonicMillis() + budget;
  q.argument = argument;
  q.modifiers = modifiers;
  q.permissions = permissions;
  q.context = std::move(context);
  q.right = std::move(right);
  q.fingerprint = fingerprint;
  q.generation = current.generation;
  std::vector<uint8_t> body;
  const auto begin = monotonicMillis();
  if (!channel.write(encode(q), q.deadline) || !channel.read(body, q.deadline))
    throw std::runtime_error(
        "Broker request timed out or disconnected (operation " +
        std::to_string(static_cast<uint32_t>(op)) + ", " +
        std::to_string(monotonicMillis() - begin) + " ms)");
  auto reply = snapshot(body, inlineCandidates);
  if (reply.sequence != sequence ||
      (current.epoch && current.epoch != reply.epoch))
    throw std::runtime_error("Stale broker response");
  if (reply.flags & reloadDeferred)
    throw BrokerReloading();
  if (reply.failed)
    throw std::runtime_error("Broker rejected request: " + reply.error);
  current = std::move(reply);
  failed = false;
  return current;
}
namespace {
std::atomic<unsigned> connectors = 0;
int moduleAnchor;
struct ConnectJob {
  Config config;
  std::shared_ptr<Connecting> state;
  HMODULE module;
  std::atomic<long> *objects;
  bool refresh;
};
void CALLBACK connectThread(PTP_CALLBACK_INSTANCE instance, void *argument) {
  CallbackMayRunLong(instance);
  HMODULE module = nullptr;
  std::atomic<long> *objects = nullptr;
  {
    std::unique_ptr<ConnectJob> job(static_cast<ConnectJob *>(argument));
    module = job->module;
    objects = job->objects;
    try {
      [&] {
        auto latest = Config::load(job->config.home, job->config.broker);
        job->state->config = latest;
        job->state->configLoaded = true;
        if (job->refresh && latest.fingerprint == job->config.fingerprint)
          return;
        job->config = std::move(latest);
        bool launched = false;
        const auto deadline = monotonicMillis() + 3000;
        while (!job->state->stopped && monotonicMillis() < deadline) {
          auto connection = Connection::connect(job->config);
          if (connection) {
            std::lock_guard lock(job->state->mutex);
            if (!job->state->stopped)
              job->state->ready = std::move(connection);
            break;
          }
          if (!launched) {
            if (!launchBroker(job->config) &&
                !std::filesystem::is_regular_file(job->config.broker))
              throw std::runtime_error("Broker executable is unavailable; "
                                       "local input remains active");
            launched = true;
          }
          Sleep(25);
        }
        if (!job->state->stopped) {
          std::lock_guard lock(job->state->mutex);
          if (!job->state->ready)
            job->state->error =
                "Broker connection unavailable; local dictionary "
                "input remains active";
        }
      }();
    } catch (const std::exception &error) {
      std::lock_guard lock(job->state->mutex);
      job->state->error = error.what();
    } catch (...) {
      std::lock_guard lock(job->state->mutex);
      job->state->error = "Broker connector failed";
    }
    job->state->finished.store(true, std::memory_order_release);
  }
  --connectors;
  if (objects)
    --*objects;
  // The loader reference covers the callback epilogue after the logical count.
  if (module)
    FreeLibraryWhenCallbackReturns(instance, module);
}
} // namespace
std::shared_ptr<Connecting> connectAsync(const Config &config, HMODULE module,
                                         std::atomic<long> *objects,
                                         bool refresh) {
  if (connectors.fetch_add(1) >= 4) {
    --connectors;
    return {};
  }
  HMODULE pinned = nullptr;
  if (module &&
      (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                           reinterpret_cast<LPCWSTR>(&moduleAnchor), &pinned) ||
       pinned != module)) {
    if (pinned)
      FreeLibrary(pinned);
    --connectors;
    return {};
  }
  try {
    auto state = std::make_shared<Connecting>();
    auto job = std::make_unique<ConnectJob>(
        ConnectJob{config, state, pinned, objects, refresh});
    if (objects)
      ++*objects;
    if (!TrySubmitThreadpoolCallback(connectThread, job.get(), nullptr)) {
      if (objects)
        --*objects;
      --connectors;
      if (pinned)
        FreeLibrary(pinned);
      return {};
    }
    job.release();
    return state;
  } catch (...) {
    --connectors;
    if (pinned)
      FreeLibrary(pinned);
    throw;
  }
}
Input::Input(const Config &settings, HMODULE dll, std::atomic<long> *counter)
    : config(settings), localDictionary(settings.dictionary),
      localEncoding(settings.encoding), module(dll), objects(counter) {
  char *error = nullptr;
  runtime.reset(tgskk_runtime_new(utf8(config.dictionary).c_str(),
                                  config.encoding.c_str(), nullptr, &error));
  if (!runtime) {
    diagnostic = error ? error : "Cannot open fallback dictionary";
    tgskk_string_free(error);
    throw std::runtime_error(diagnostic);
  }
  tgskk_string_free(error);
  error = nullptr;
  local.reset(tgskk_session_new(runtime.get(), &error));
  if (!local) {
    diagnostic = error ? error : "Cannot create fallback input";
    tgskk_string_free(error);
    throw std::runtime_error(diagnostic);
  }
  tgskk_string_free(error);
  startConnecting();
}
Input::~Input() {
  if (connecting)
    connecting->stopped = true;
  remote.reset();
  connecting.reset();
}
const TgskkView *Input::remoteView() {
  const auto &s = remote->current;
  if (!s.error.empty() && s.error != lastRemoteError) {
    diagnostic = s.error;
    lastRemoteError = s.error;
  }
  cached = {uint8_t(bool(s.flags & 1)),
            uint8_t(bool(s.flags & 2)),
            uint8_t(bool(s.flags & 4)),
            uint8_t(bool(s.flags & 8)),
            s.selected,
            s.count,
            s.committed.c_str(),
            s.preedit.c_str(),
            s.label.c_str()};
  return &cached;
}
const TgskkView *Input::view() {
  return remote ? remoteView() : tgskk_session_view(local.get());
}
void Input::startConnecting() {
  if (connecting)
    connecting->stopped = true;
  connecting = connectAsync(config, module, objects);
  retryAt = 0;
  if (!connecting)
    diagnostic =
        "Broker connector is busy; local dictionary input remains active";
}
void Input::adopt() {
  const auto *current = view();
  if (!current || *current->preedit)
    return;
  if (connecting) {
    std::unique_lock lock(connecting->mutex, std::try_to_lock);
    if (!lock)
      return;
    if (!connecting->error.empty()) {
      diagnostic = std::move(connecting->error);
      connecting->error.clear();
    }
    auto ready = std::move(connecting->ready);
    const bool finished = connecting->finished.load(std::memory_order_acquire);
    lock.unlock();
    if (ready) {
      const auto localMode = modeFromLabel(current->mode_label);
      try {
        if (ready->current.mode != localMode)
          ready->perform(Op::Mode, localMode);
        auto nextConfig = connecting->config;
        config = std::move(nextConfig);
        remote = std::move(ready);
        connecting->stopped = true;
        connecting.reset();
        retryAt = 0;
        retryDelay = 1000;
        return;
      } catch (const std::exception &error) {
        diagnostic = error.what();
        connecting->stopped = true;
        connecting.reset();
      }
    } else if (!finished)
      return;
    else {
      if (!remote && connecting->configLoaded &&
          connecting->config.fingerprint != config.fingerprint &&
          (connecting->config.fingerprint != rejectedConfig ||
           monotonicMillis() >= configRetryAt)) {
        auto previous = config;
        try {
          config = connecting->config;
          updateFallback(modeFromLabel(current->mode_label));
          retryDelay = 1000;
        } catch (const std::exception &error) {
          config = std::move(previous);
          rejectedConfig = connecting->config.fingerprint;
          configRetryAt = monotonicMillis() + 30000;
          diagnostic = error.what();
        }
      }
      connecting.reset();
    }
  }
  if (remote)
    return;
  const auto now = monotonicMillis();
  if (!retryAt)
    retryAt = now + retryDelay;
  if (now < retryAt)
    return;
  startConnecting();
  retryDelay = std::min<uint64_t>(30000, retryDelay * 2);
}
void Input::refresh() {
  adopt();
  const auto *current = view();
  const auto now = monotonicMillis();
  if (!current || *current->preedit || connecting || now < refreshAt)
    return;
  refreshAt = now + 1000;
  connecting = connectAsync(config, module, objects, true);
}
void Input::updateFallback(uint8_t mode) {
  if (config.dictionary != localDictionary ||
      config.encoding != localEncoding) {
    auto nextDictionary = config.dictionary;
    auto nextEncoding = config.encoding;
    char *error = nullptr;
    std::unique_ptr<TgskkRuntime, decltype(&tgskk_runtime_free)> nextRuntime(
        tgskk_runtime_new(utf8(config.dictionary).c_str(),
                          config.encoding.c_str(), nullptr, &error),
        tgskk_runtime_free);
    std::string message = error ? error : "Cannot reload fallback dictionary";
    tgskk_string_free(error);
    if (!nextRuntime)
      throw std::runtime_error(message);
    error = nullptr;
    std::unique_ptr<TgskkSession, decltype(&tgskk_session_free)> nextInput(
        tgskk_session_new(nextRuntime.get(), &error), tgskk_session_free);
    message = error ? error : "Cannot reload fallback session";
    tgskk_string_free(error);
    if (!nextInput || !tgskk_session_set_mode(nextInput.get(), mode))
      throw std::runtime_error(message);
    local = std::move(nextInput);
    runtime = std::move(nextRuntime);
    localDictionary = std::move(nextDictionary);
    localEncoding = std::move(nextEncoding);
  }
}
void Input::fail(const std::string &message) {
  const auto oldMode =
      remote ? remote->current.mode
             : modeFromLabel(tgskk_session_view(local.get())->mode_label);
  diagnostic = message;
  transportFailed = true;
  remote.reset();
  try {
    updateFallback(static_cast<uint8_t>(oldMode));
  } catch (const std::exception &error) {
    diagnostic += "; " + std::string(error.what());
  }
  tgskk_session_set_mode(local.get(), static_cast<uint8_t>(oldMode));
  retryDelay = 1000;
  startConnecting();
}
const TgskkView *Input::action(Op op, uint32_t argument, uint32_t modifiers) {
  // Context permissions were computed against the current configuration.
  if (op == Op::Key && !permissions)
    adopt();
  if (remote) {
    try {
      remote->perform(op, argument, modifiers, op == Op::Key ? permissions : 0,
                      op == Op::Key ? left : "", op == Op::Key ? right : "");
      return remoteView();
    } catch (const BrokerReloading &error) {
      const bool idle = remote->current.preedit.empty();
      fail(error.what());
      if (!idle || op == Op::Choose || op == Op::Select)
        return nullptr;
      // The broker explicitly guarantees that this request was not executed.
      transportFailed = false;
    } catch (const std::exception &error) {
      fail(error.what());
      return nullptr;
    }
  }
  switch (op) {
  case Op::Key:
    return tgskk_session_key(local.get(), argument, modifiers, 0);
  case Op::Choose:
    return tgskk_session_choose(local.get(), argument);
  case Op::Select:
    return tgskk_session_select(local.get(), argument);
  case Op::Reset:
    return tgskk_session_reset(local.get());
  case Op::Mode:
    return tgskk_session_set_mode(local.get(), static_cast<uint8_t>(argument));
  default:
    return nullptr;
  }
}
const TgskkView *Input::key(uint32_t symbol, uint32_t modifiers) {
  return action(Op::Key, symbol, modifiers);
}
const TgskkView *Input::choose(size_t index) {
  return index <= UINT32_MAX ? action(Op::Choose, static_cast<uint32_t>(index))
                             : nullptr;
}
const TgskkView *Input::select(size_t index) {
  return index <= UINT32_MAX ? action(Op::Select, static_cast<uint32_t>(index))
                             : nullptr;
}
const TgskkView *Input::reset() {
  left.clear();
  right.clear();
  permissions = 0;
  return action(Op::Reset);
}
const TgskkView *Input::setMode(uint8_t mode) {
  left.clear();
  right.clear();
  permissions = 0;
  return action(Op::Mode, mode);
}
bool Input::mayProcess(uint32_t symbol, uint32_t modifiers) {
  if (remote)
    return remote->current.eligible(symbol, modifiers);
  const auto *current = tgskk_session_view(local.get());
  if (current &&
      isAsciiPassthrough(modeFromLabel(current->mode_label), symbol, modifiers))
    return false;
  return tgskk_session_may_process(local.get(), symbol, modifiers) > 0;
}

const char *Input::candidate(size_t index, bool annotation) {
  if (!remote)
    return annotation ? tgskk_candidate_annotation(local.get(), index)
                      : tgskk_candidate_text(local.get(), index);
  const auto &s = remote->current;
  if (index < s.start || index - s.start >= s.candidates.size())
    return nullptr;
  const auto &c = s.candidates[index - s.start];
  return annotation ? c.annotation.c_str() : c.text.c_str();
}
void Input::context(std::string leftText, std::string rightText, bool ai,
                    bool network) {
  left = ai ? std::move(leftText) : "";
  right = ai ? std::move(rightText) : "";
  permissions = 4 | (ai ? 1 : 0) | (network ? 2 : 0);
}
} // namespace tgskk::windows
