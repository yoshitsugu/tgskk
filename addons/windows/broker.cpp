// SPDX-License-Identifier: GPL-3.0-or-later
#include "broker_queue.h"
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <deque>
#include <map>
#include <mutex>
#include <optional>
#include <thread>
#include <tuple>
#include <utility>

using namespace tgskk::windows;
namespace {
Handle shutdownEvent;
BOOL WINAPI interrupt(DWORD) {
  if (shutdownEvent)
    SetEvent(shutdownEvent.value);
  return TRUE;
}
void serve(const std::shared_ptr<Client> &client,
           const std::shared_ptr<Queue> &queue, const Identity &identity) {
  try {
    if (!acceptPipe(client->channel.handle(), client->canceled.value, 1000))
      throw std::runtime_error("Client setup expired");
    verifyPeer(client->channel.handle(), true, identity);
    std::vector<uint8_t> bytes;
    while (!queue->stopped && client->channel.read(bytes, UINT64_MAX)) {
      auto q = request(bytes);
      const auto now = monotonicMillis();
      if (q.sequence != client->next++ || q.deadline <= now)
        break;
      q.deadline = std::min<uint64_t>(
          q.deadline, now + (q.op == Op::Hello ? helloBudget : keyBudget));
      const auto deadline = q.deadline;
      {
        std::lock_guard lock(client->mutex);
        client->ready = false;
      }
      if (!queue->submit({client, std::move(q)}))
        break;
      std::unique_lock lock(client->mutex);
      const auto waiting = monotonicMillis();
      const auto remaining = deadline > waiting ? deadline - waiting : 0;
      if (!client->wake.wait_for(lock, std::chrono::milliseconds(remaining),
                                 [&] {
                                   return client->ready || !client->live ||
                                          queue->stopped;
                                 }) ||
          !client->ready)
        break;
      auto reply = std::move(client->reply);
      lock.unlock();
      if (!client->channel.write(reply, deadline))
        break;
    }
  } catch (
      ...) { /* A malformed or disconnected client loses only its session. */
  }
  client->live = false;
  --queue->connections;
  queue->wake.notify_one();
}
void acceptClients(Handle listener, PipeSecurity &security,
                   const Identity &identity, const std::wstring &name,
                   const std::shared_ptr<Queue> &queue) {
  while (!queue->stopped && acceptPipe(listener.value, shutdownEvent.value)) {
    std::shared_ptr<Client> client;
    try {
      verifyPeer(listener.value, true, identity);
      if (queue->connections >= 32)
        throw std::runtime_error("Connection limit");
      const auto nonce = randomId();
      const auto dedicated = name + L"-c-" + std::to_wstring(nonce);
      client = std::make_shared<Client>(makePipe(dedicated, security));
      ++queue->connections;
      {
        std::lock_guard lock(queue->mutex);
        std::erase_if(queue->clients,
                      [](const auto &c) { return c.expired(); });
        queue->clients.push_back(client);
      }
      try {
        std::thread(serve, client, queue, identity).detach();
      } catch (...) {
        --queue->connections;
        throw;
      }
      HANDLE copy = nullptr;
      if (!DuplicateHandle(GetCurrentProcess(), listener.value,
                           GetCurrentProcess(), &copy, 0, FALSE,
                           DUPLICATE_SAME_ACCESS))
        throw std::runtime_error("Rendezvous handle");
      Channel rendezvous{Handle(copy)};
      Writer body;
      body.string(utf8(dedicated));
      const auto deadline = monotonicMillis() + 100;
      std::vector<uint8_t> ack;
      if (!rendezvous.write(body.data, deadline) ||
          !rendezvous.read(ack, deadline))
        throw std::runtime_error("Rendezvous expired");
      Reader confirmation(ack);
      if (confirmation.u64() != nonce)
        throw std::runtime_error("Invalid rendezvous acknowledgement");
      confirmation.end();
    } catch (...) {
      if (client) {
        client->live = false;
        SetEvent(client->canceled.value);
        CancelIoEx(client->channel.handle(), nullptr);
      }
    }
    DisconnectNamedPipe(listener.value);
  }
  // An unexpected listener failure must not leave an unreachable broker alive.
  if (!queue->stopped &&
      WaitForSingleObject(shutdownEvent.value, 0) != WAIT_OBJECT_0) {
    std::fputs("tgskk broker: listener stopped; shutting down\n", stderr);
    SetEvent(shutdownEvent.value);
  }
}
struct Dictionaries {
  Config config;
  const std::wstring userPath;
  Handle lock;
  TgskkRuntime *runtime = nullptr;
  TgskkSaver *saver = nullptr;
  uint64_t lastSave = 0;
  std::string diagnostic, saveError;
  void takeError(char *error) {
    if (error) {
      diagnostic = error;
      tgskk_string_free(error);
    }
  }
  explicit Dictionaries(Config settings)
      : config(std::move(settings)), userPath(config.userDictionary) {
    try {
      std::filesystem::create_directories(
          std::filesystem::path(config.userDictionary).parent_path());
      Handle user(CreateFileW(
          config.userDictionary.c_str(), GENERIC_READ | GENERIC_WRITE,
          FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr,
          OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr));
      if (user) {
        BY_HANDLE_FILE_INFORMATION information{};
        if (!GetFileInformationByHandle(user.value, &information) ||
            information.nNumberOfLinks != 1)
          throw std::runtime_error("User dictionary must not have hard links");
        std::wstring final(32768, L'\0');
        const auto n = GetFinalPathNameByHandleW(
            user.value, final.data(), static_cast<DWORD>(final.size()),
            FILE_NAME_NORMALIZED);
        if (!n || n >= final.size())
          throw std::runtime_error("User dictionary canonical path");
        final.resize(n);
        config.userDictionary = final;
        lock = Handle(CreateFileW((final + L".lock").c_str(),
                                  GENERIC_READ | GENERIC_WRITE,
                                  FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr,
                                  OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr));
        OVERLAPPED overlap{};
        if (lock &&
            !LockFileEx(lock.value,
                        LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0,
                        1, 0, &overlap))
          lock = Handle();
      }
      user = Handle();
      char *error = nullptr;
      const auto dictionary = utf8(config.dictionary),
                 writable = utf8(config.userDictionary);
      runtime = tgskk_runtime_new(dictionary.c_str(), config.encoding.c_str(),
                                  lock ? writable.c_str() : nullptr, &error);
      takeError(std::exchange(error, nullptr));
      if (!runtime)
        throw std::runtime_error("Cannot initialize broker dictionary: " +
                                 diagnostic);
      if (lock) {
        saver = tgskk_runtime_saver_new(runtime, &error);
        takeError(std::exchange(error, nullptr));
        if (!saver)
          throw std::runtime_error("Cannot initialize dictionary writer: " +
                                   diagnostic);
      } else {
        diagnostic = "User dictionary is read-only: another writer or a file "
                     "access error";
        if (std::filesystem::exists(config.userDictionary)) {
          tgskk_runtime_prepend_readonly(runtime, writable.c_str(), "utf-8",
                                         &error);
          takeError(std::exchange(error, nullptr));
        }
      }
    } catch (...) {
      tgskk_saver_free(saver);
      tgskk_runtime_free(runtime);
      throw;
    }
  }
  ~Dictionaries() {
    tgskk_saver_free(saver);
    tgskk_runtime_free(runtime);
  }
  void save(bool force, uint64_t lastInput) {
    const auto now = monotonicMillis();
    if (saver &&
        (force || (now - lastInput >= 1000 && now - lastSave >= 1000))) {
      lastSave = now;
      char *error = nullptr;
      if (!tgskk_saver_tick(saver, &error)) {
        saveError = error ? error : "Dictionary save failed";
        tgskk_string_free(error);
      } else if (tgskk_saver_idle(saver))
        saveError.clear();
    }
  }
};
struct Services {
  Config config;
  TgskkZenz *ai = nullptr;
  TgskkSkkserv *server = nullptr;
  std::string diagnostic;
  void takeError(char *error) {
    if (error) {
      diagnostic = error;
      tgskk_string_free(error);
    }
  }
  explicit Services(Config settings) : config(std::move(settings)) {
    char *error = nullptr;
    try {
      if (config.ai) {
        if (std::filesystem::exists(config.worker) &&
            std::filesystem::exists(config.model)) {
          ai = tgskk_zenz_new(utf8(config.worker).c_str(),
                              utf8(config.model).c_str(), &error);
          takeError(std::exchange(error, nullptr));
        } else
          diagnostic = "AI worker or model is unavailable; dictionary order "
                       "remains active";
      }
      if (config.server == "Google" && std::filesystem::exists(config.helper))
        server =
            tgskk_skkserv_google_new(utf8(config.helper).c_str(), 0, &error);
      else if (config.server == "Custom")
        server =
            tgskk_skkserv_custom_new(config.host.c_str(), config.port,
                                     config.serverEncoding.c_str(), 0, &error);
      else if (config.server == "Google")
        diagnostic = "Google SKK helper is unavailable";
      takeError(std::exchange(error, nullptr));

    } catch (...) {
      tgskk_zenz_free(ai);
      tgskk_skkserv_free(server);
      throw;
    }
  }
  bool matches(const Config &other) const {
    return std::tie(config.ai, config.worker, config.model, config.server,
                    config.helper, config.host, config.port,
                    config.serverEncoding) ==
           std::tie(other.ai, other.worker, other.model, other.server,
                    other.helper, other.host, other.port, other.serverEncoding);
  }
  ~Services() {
    tgskk_zenz_free(ai);
    tgskk_skkserv_free(server);
  }
};
struct Engine {
  Config config;
  std::shared_ptr<Dictionaries> dictionaries;
  std::shared_ptr<Services> services;
  TgskkRuntime *runtime = nullptr;
  Engine(Config settings, std::shared_ptr<Dictionaries> shared,
         std::shared_ptr<Services> assistance)
      : config(std::move(settings)), dictionaries(std::move(shared)),
        services(std::move(assistance)) {
    char *error = nullptr;
    try {
      if (config.dictionary != dictionaries->config.dictionary ||
          config.encoding != dictionaries->config.encoding) {
        runtime = tgskk_runtime_with_dictionary(
            dictionaries->runtime, utf8(config.dictionary).c_str(),
            config.encoding.c_str(), &error);
        std::string message = error ? error : "Cannot reload dictionary";
        tgskk_string_free(error);
        if (!runtime)
          throw std::runtime_error(message);
      }
      if (!services)
        services = std::make_shared<Services>(config);
    } catch (...) {
      tgskk_runtime_free(runtime);
      throw;
    }
  }
  TgskkRuntime *dictionary() const {
    return runtime ? runtime : dictionaries->runtime;
  }
  ~Engine() { tgskk_runtime_free(runtime); }
};
struct Session {
  std::shared_ptr<Engine> engine;
  TgskkSession *input;
  std::shared_ptr<Client> client;
  uint64_t generation = 1, pendingSince = 0;
  std::string pendingReading;
  Session(TgskkSession *value, std::shared_ptr<Client> owner,
          std::shared_ptr<Engine> settings)
      : engine(std::move(settings)), input(value), client(std::move(owner)) {}
  ~Session() { tgskk_session_free(input); }
};
class ReloadWaiting : public std::runtime_error {
public:
  ReloadWaiting()
      : std::runtime_error("Dictionary reload waits for pending input") {}
};
class Core {
  Config origin;
  std::shared_ptr<Queue> queue;
  // Retain the latest resources between client reconnects.
  std::shared_ptr<Engine> warmEngine;
  std::vector<std::weak_ptr<Engine>> engines;
  std::vector<std::shared_ptr<Dictionaries>> stores;
  std::map<Client *, std::unique_ptr<Session>> sessions;
  Client *foreground = nullptr;
  uint64_t epoch, lastKey = 0, processedKeys = 0;
  uint64_t failedFingerprint = 0, retryAfter = 0;
  std::string reloadError;
  std::shared_ptr<Engine> settings(uint64_t fingerprint) {
    for (auto &weak : engines)
      if (auto engine = weak.lock();
          engine && engine->config.fingerprint == fingerprint)
        return engine;
    if (failedFingerprint == fingerprint && monotonicMillis() < retryAfter)
      throw std::runtime_error(reloadError);
    try {
      const auto config = Config::load(origin.home, origin.broker);
      if (config.fingerprint != fingerprint)
        throw std::runtime_error("Settings changed during connection; retry");
      std::shared_ptr<Dictionaries> dictionaries;
      for (const auto &store : stores)
        if (CompareStringOrdinal(store->userPath.c_str(), -1,
                                 config.userDictionary.c_str(), -1,
                                 TRUE) == CSTR_EQUAL) {
          dictionaries = store;
          break;
        }
      std::optional<ReloadPause> pause;
      // Loading another dictionary must not time out a live composition's keys.
      if (!dictionaries ||
          config.dictionary != dictionaries->config.dictionary ||
          config.encoding != dictionaries->config.encoding) {
        for (const auto &[client, session] : sessions) {
          const auto *view = tgskk_session_view(session->input);
          if (!view || *view->preedit)
            throw ReloadWaiting();
        }
        pause.emplace(*queue);
      }
      if (!dictionaries) {
        dictionaries = std::make_shared<Dictionaries>(config);
        stores.push_back(dictionaries);
      }
      std::shared_ptr<Services> services;
      for (const auto &weak : engines)
        if (auto engine = weak.lock();
            engine && engine->services->matches(config)) {
          services = engine->services;
          break;
        }
      auto engine = std::make_shared<Engine>(config, std::move(dictionaries),
                                             std::move(services));
      engines.push_back(engine);
      warmEngine = engine;
      return engine;
    } catch (const ReloadWaiting &) {
      throw;
    } catch (const std::exception &error) {
      failedFingerprint = fingerprint;
      retryAfter = monotonicMillis() + 30000;
      reloadError = error.what();
      throw;
    }
  }

public:
  Core(const Config &config, std::shared_ptr<Queue> transport)
      : origin(config), queue(std::move(transport)), epoch(queue->epoch) {
    settings(config.fingerprint);
  }
  ~Core() { sessions.clear(); }
  void tick() {
    std::erase_if(sessions, [&](const auto &item) {
      if (!item.second->client->live) {
        if (foreground == item.first)
          foreground = nullptr;
        return true;
      }
      return false;
    });
    scheduleSave(false);
    std::erase_if(engines, [](const auto &weak) { return weak.expired(); });
    std::erase_if(stores, [](const auto &store) {
      return store.use_count() == 1 && store->saveError.empty() &&
             (!store->saver || tgskk_saver_idle(store->saver));
    });
  }
  void scheduleSave(bool force) {
    for (const auto &store : stores)
      store->save(force, lastKey);
  }
  Snapshot process(const Job &job) {
    const auto &q = job.request;
    Snapshot reply;
    reply.sequence = q.sequence;
    reply.epoch = epoch;
    try {
      if (q.op == Op::Hello) {
        if (sessions.contains(job.client.get()) || q.sequence != 1 ||
            q.permissions || !q.context.empty() || !q.right.empty())
          throw std::runtime_error("Invalid hello");
        auto engine = settings(q.fingerprint);
        char *error = nullptr;
        auto *input = tgskk_session_new(engine->dictionary(), &error);
        if (!input) {
          std::string message = error ? error : "Cannot create broker session";
          tgskk_string_free(error);
          throw std::runtime_error(message);
        }
        auto session = std::make_unique<Session>(input, job.client, engine);
        tgskk_session_set_zenz(input, engine->services->ai);
        tgskk_session_set_skkserv(input, engine->services->server);
        tgskk_session_defer_missing(input, 1);
        sessions.emplace(job.client.get(), std::move(session));
      }
      auto found = sessions.find(job.client.get());
      if (found == sessions.end())
        throw std::runtime_error("Session is not initialized");
      auto &session = *found->second;
      auto &engine = *session.engine;
      const auto &config = engine.config;
      auto &dictionary = *engine.dictionaries;
      if (q.fingerprint != config.fingerprint)
        throw std::runtime_error("Session configuration mismatch");
      const TgskkView *view = tgskk_session_view(session.input);
      if (!view)
        throw std::runtime_error("Invalid input session");
      const auto oldReading = std::string(view->preedit);
      if (q.op == Op::Key) {
        lastKey = monotonicMillis();
        ++processedKeys;
        if (foreground != job.client.get()) {
          if (auto previous = sessions.find(foreground);
              previous != sessions.end())
            tgskk_session_context(previous->second->input, "", "", 0);
          foreground = job.client.get();
        }
        const bool active = (q.permissions & 4) != 0;
        tgskk_session_context(session.input, active ? q.context.c_str() : "",
                              active ? q.right.c_str() : "",
                              active && (q.permissions & 1));
        bool network = active && (q.permissions & 2);
        auto modifiers = q.modifiers;
        if (session.pendingSince && q.argument == ' ' &&
            (q.modifiers == TGSKK_MOD_SHIFT ||
             monotonicMillis() - session.pendingSince >= 3000)) {
          network = false;
          if (q.modifiers == TGSKK_MOD_SHIFT)
            modifiers = 0;
        }
        tgskk_session_network_allowed(session.input, network);
        view = tgskk_session_key(session.input, q.argument, modifiers, 0);
        const bool pending =
            network && tgskk_session_lookup_pending(session.input);
        if (pending && view) {
          if (!session.pendingSince || session.pendingReading != oldReading)
            session.pendingSince = monotonicMillis();
          session.pendingReading = view ? view->preedit : oldReading;
        } else {
          session.pendingSince = 0;
          session.pendingReading.clear();
        }
        ++session.generation;
      } else if (q.op == Op::Choose || q.op == Op::Select) {
        if (q.generation != session.generation || !view->selecting ||
            q.argument >= view->candidate_count)
          throw std::runtime_error("Stale candidate selection");
        view = q.op == Op::Choose
                   ? tgskk_session_choose(session.input, q.argument)
                   : tgskk_session_select(session.input, q.argument);
        ++session.generation;
      } else if (q.op == Op::Reset || q.op == Op::Mode) {
        if (q.op == Op::Mode && q.argument > 4)
          throw std::runtime_error("Invalid input mode");
        tgskk_session_context(session.input, "", "", 0);
        tgskk_session_network_allowed(session.input, 0);
        view = q.op == Op::Reset
                   ? tgskk_session_reset(session.input)
                   : tgskk_session_set_mode(session.input,
                                            static_cast<uint8_t>(q.argument));
        session.pendingSince = 0;
        ++session.generation;
      } else if (q.op == Op::Flush)
        scheduleSave(true);
      if (!view)
        throw std::runtime_error(tgskk_session_error(session.input));
      if (view->candidate_count > 1000000)
        throw std::runtime_error("Candidate count limit");
      reply.generation = session.generation;
      reply.count = static_cast<uint32_t>(view->candidate_count);
      reply.selected = static_cast<uint32_t>(view->selected);
      reply.start = static_cast<uint32_t>(
          tgskk::candidatePageStart(reply.selected, config.inlineCandidates));
      reply.mode = modeFromLabel(view->mode_label);
      reply.preedit = view->preedit;
      reply.label = view->mode_label;
      const bool action = q.op == Op::Key || q.op == Op::Choose ||
                          q.op == Op::Select || q.op == Op::Reset ||
                          q.op == Op::Mode;
      reply.flags = (action && view->consumed ? 1 : 0) |
                    (action && view->mode_changed ? 2 : 0) |
                    (view->selecting ? 4 : 0) | (view->zenz_applied ? 8 : 0) |
                    (session.pendingSince ? 16 : 0);
      if (action)
        reply.committed = view->committed;
      for (size_t i = reply.start;
           i < view->candidate_count && i < reply.start + 7; ++i) {
        const auto *annotation = tgskk_candidate_annotation(session.input, i);
        reply.candidates.push_back({tgskk_candidate_text(session.input, i),
                                    annotation ? annotation : ""});
      }
      const auto recordEligibility = [&](uint32_t symbol, uint32_t modifiers) {
        const auto index = eligibilityIndex(symbol, modifiers);
        if (index < 0)
          return;
        const auto eligible =
            tgskk_session_may_process(session.input, symbol, modifiers);
        if (eligible < 0)
          throw std::runtime_error(tgskk_session_error(session.input));
        if (eligible)
          reply.eligibility[index / 8] |= 1 << (index % 8);
      };
      for (uint32_t modifiers : {0U, 1U, 4U, 5U}) {
        for (uint32_t symbol = 0x20; symbol <= 0x7e; ++symbol)
          recordEligibility(symbol, modifiers);
        for (uint32_t symbol : specialKeys)
          recordEligibility(symbol, modifiers);
      }
      reply.processedKeys = processedKeys;
      reply.ai =
          engine.services->ai ? tgskk_zenz_status(engine.services->ai) : 3;
      reply.persistence = !dictionary.saver || !dictionary.saveError.empty() ? 3
                          : tgskk_saver_idle(dictionary.saver) ? 2
                                                               : 1;
      engine.services->takeError(tgskk_zenz_take_error(engine.services->ai));
      engine.services->takeError(
          tgskk_skkserv_take_error(engine.services->server));
      reply.error = !dictionary.saveError.empty() ? dictionary.saveError
                    : !engine.services->diagnostic.empty()
                        ? engine.services->diagnostic
                        : dictionary.diagnostic;
    } catch (const std::exception &error) {
      reply.failed = true;
      reply.error = error.what();
    }
    return reply;
  }
  bool flush() {
    const auto deadline = monotonicMillis() + 5000;
    const auto idle = [&] {
      for (const auto &store : stores)
        if (store->saver && !tgskk_saver_idle(store->saver))
          return false;
      return true;
    };
    do {
      scheduleSave(true);
      if (idle())
        return true;
      Sleep(1);
    } while (monotonicMillis() < deadline);
    return false;
  }
};
} // namespace
int wmain(int argc, wchar_t **argv) {
  try {
    const auto identity = Identity::current();
    std::wstring home = homeDirectory();
    if (argc == 3 && std::wstring(argv[1]) == L"--home")
      home = canonicalPath(argv[2]);
    else if (argc != 1)
      throw std::runtime_error(
          "Usage: tgskk-broker [--home absolute-directory]");
    const auto config = Config::load(home, executablePath());
    PipeSecurity security(identity);
    const auto name = endpoint(identity, config.home, config.broker);
    Handle listener;
    try {
      listener = makePipe(name, security);
    } catch (const std::exception &error) {
      std::fprintf(stderr, "tgskk broker endpoint: %s\n", error.what());
      return 2;
    }
    Handle children(CreateJobObjectW(nullptr, nullptr));
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!children ||
        !SetInformationJobObject(children.value,
                                 JobObjectExtendedLimitInformation, &limits,
                                 sizeof(limits)) ||
        !AssignProcessToJobObject(children.value, GetCurrentProcess()))
      throw std::runtime_error("Cannot contain helper processes");
    // Keep the job until kernel process teardown; closing it also kills us.
    children.value = INVALID_HANDLE_VALUE;
    auto queue = std::make_shared<Queue>();
    Core core(config, queue);
    // The single input owner must not compete at the scorer's CPU priority.
    if (!SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL))
      throw std::runtime_error("Cannot prioritize broker input thread");
    shutdownEvent = Handle(CreateEventW(nullptr, TRUE, FALSE, nullptr));
    if (!shutdownEvent)
      throw std::runtime_error("Cannot create shutdown event");
    SetConsoleCtrlHandler(interrupt, TRUE);
    std::thread acceptor(acceptClients, std::move(listener), std::ref(security),
                         identity, name, queue);
    while (WaitForSingleObject(shutdownEvent.value, 0) != WAIT_OBJECT_0) {
      core.tick();
      Job job;
      {
        std::unique_lock lock(queue->mutex);
        if (!queue->wake.wait_for(lock, std::chrono::milliseconds(20),
                                  [&] { return !queue->jobs.empty(); }))
          continue;
        job = std::move(queue->jobs.front());
        queue->jobs.pop_front();
      }
      if (!job.client->live || monotonicMillis() >= job.request.deadline)
        continue;
      std::vector<uint8_t> result;
      try {
        result = encode(core.process(job));
      } catch (...) {
        job.client->live = false;
        job.client->wake.notify_one();
        continue;
      }
      {
        std::lock_guard lock(job.client->mutex);
        if (job.client->live && monotonicMillis() < job.request.deadline) {
          job.client->reply = std::move(result);
          job.client->ready = true;
          job.client->wake.notify_one();
        }
      }
    }
    queue->stopped = true;
    {
      std::lock_guard lock(queue->mutex);
      for (const auto &weak : queue->clients)
        if (const auto client = weak.lock()) {
          client->live = false;
          SetEvent(client->canceled.value);
          CancelIoEx(client->channel.handle(), nullptr);
          client->wake.notify_one();
        }
    }
    queue->wake.notify_all();
    acceptor.join();
    if (!core.flush())
      throw std::runtime_error(
          "Dictionary flush did not finish; unsaved learning remains");
    return 0;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "tgskk broker: %s\n", error.what());
    return 1;
  }
}
