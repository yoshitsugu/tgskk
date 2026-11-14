// SPDX-License-Identifier: GPL-3.0-or-later
#include "../broker_client.h"
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <exception>
#include <fstream>
#include <functional>
#include <io.h>
#include <thread>
#include <winsock2.h>
#include <ws2tcpip.h>
using namespace tgskk::windows;
namespace {
void require(bool okay, const std::string &message) {
  if (!okay)
    throw std::runtime_error(message);
}
struct Directory {
  std::filesystem::path path;
  Directory() {
    wchar_t temp[MAX_PATH];
    require(GetTempPathW(MAX_PATH, temp) != 0, "temp path");
    path = std::filesystem::path(temp) /
           (L"tgskk-broker-test-" + std::to_wstring(randomId()));
    std::filesystem::create_directories(path);
  }
  ~Directory() {
    std::error_code ignored;
    std::filesystem::remove_all(path, ignored);
  }
};
void write(const std::filesystem::path &path, const std::string &text) {
  std::ofstream file(path, std::ios::binary);
  file << text;
  require(bool(file), "write fixture");
}
std::string read(const std::filesystem::path &path) {
  std::ifstream file(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(file), {}};
}
Handle startLogged(const std::wstring &program, std::wstring command,
                   const std::filesystem::path &logPath,
                   DWORD flags = CREATE_NO_WINDOW) {
  SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
  Handle log(CreateFileW(logPath.c_str(), GENERIC_WRITE,
                         FILE_SHARE_READ | FILE_SHARE_WRITE, &attributes,
                         CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr));
  Handle input(CreateFileW(L"NUL", GENERIC_READ,
                           FILE_SHARE_READ | FILE_SHARE_WRITE, &attributes,
                           OPEN_EXISTING, 0, nullptr));
  require(log && input, "broker test stdio");
  SIZE_T length = 0;
  InitializeProcThreadAttributeList(nullptr, 1, 0, &length);
  std::vector<uint8_t> storage(length);
  auto *list = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
  require(InitializeProcThreadAttributeList(list, 1, 0, &length),
          "broker handle list");
  HANDLE inherited[] = {input.value, log.value};
  require(UpdateProcThreadAttribute(list, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                                    inherited, sizeof(inherited), nullptr,
                                    nullptr),
          "broker handle whitelist");
  STARTUPINFOEXW startup{};
  startup.StartupInfo.cb = sizeof(startup);
  startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
  startup.StartupInfo.hStdInput = input.value;
  startup.StartupInfo.hStdOutput = startup.StartupInfo.hStdError = log.value;
  startup.lpAttributeList = list;
  PROCESS_INFORMATION child{};
  const bool created =
      CreateProcessW(program.c_str(), command.data(), nullptr, nullptr, TRUE,
                     flags | EXTENDED_STARTUPINFO_PRESENT, nullptr, nullptr,
                     &startup.StartupInfo, &child);
  const auto error = GetLastError();
  DeleteProcThreadAttributeList(list);
  require(created, "start broker: " + std::to_string(error));
  CloseHandle(child.hThread);
  return Handle(child.hProcess);
}
struct Broker {
  Handle process;
  Config config;
  std::filesystem::path logPath;
  Broker(const std::filesystem::path &home, const std::wstring &program,
         const std::wstring &dictionary, const std::string &extra = {}) {
    std::filesystem::create_directories(home);
    write(home / L"windows.ini",
          "Dictionary=" + utf8(dictionary) +
              "\nEncoding=utf-8\nAI=false\nSkkserv=Disabled\n" + extra);
    config = Config::load(home.wstring(), program);
    auto command = L"\"" + program + L"\" --home \"" + home.wstring() + L"\"";
    logPath = home / L"broker.log";
    process = startLogged(program, command, logPath);
  }
  void stop() {
    if (process) {
      TerminateProcess(process.value, 0);
      require(WaitForSingleObject(process.value, 5000) == WAIT_OBJECT_0,
              "stop broker");
      process = Handle();
    }
  }
  ~Broker() {
    if (std::uncaught_exceptions())
      std::fprintf(stderr, "broker diagnostic: %s\n", read(logPath).c_str());
    stop();
  }
  std::unique_ptr<Connection> connect() {
    const auto deadline = monotonicMillis() + 10000;
    std::string error;
    while (monotonicMillis() < deadline) {
      try {
        if (auto connection = Connection::connect(config))
          return connection;
      } catch (const std::exception &e) {
        error = e.what();
      }
      DWORD code = 0;
      require(GetExitCodeProcess(process.value, &code), "broker exit status");
      require(code == STILL_ACTIVE, "broker exited during setup (" +
                                        std::to_string(code) + ") " + error +
                                        "\n" + read(logPath));
      Sleep(10);
    }
    throw std::runtime_error("broker setup timed out: " + error + "\n" +
                             read(logPath));
  }
};
const Snapshot &key(Connection &connection, uint32_t symbol,
                    uint32_t modifiers = 0, const std::string &context = {},
                    uint32_t permissions = 4) {
  return connection.perform(Op::Key, symbol, modifiers, permissions, context);
}
void type(Connection &connection, const std::string &text,
          const std::string &context = {}, uint32_t permissions = 4,
          DWORD interval = 0) {
  for (unsigned char symbol : text) {
    key(connection, symbol,
        symbol >= 'A' && symbol <= 'Z' ? TGSKK_MOD_SHIFT : 0, context,
        permissions);
    if (interval)
      Sleep(interval);
  }
}
void flush(Connection &connection) {
  const auto deadline = monotonicMillis() + 5000;
  while (connection.perform(Op::Flush).persistence != 2) {
    require(monotonicMillis() < deadline,
            "dictionary flush timed out: " + connection.current.error);
    Sleep(1);
  }
}
std::string first(const Connection &connection) {
  require(connection.current.start == 0 &&
              !connection.current.candidates.empty(),
          "no first candidate");
  return connection.current.candidates[0].text;
}
void waitUntil(const std::function<bool()> &condition, const char *message,
               DWORD timeout = 3000) {
  const auto deadline = monotonicMillis() + timeout;
  while (!condition()) {
    require(monotonicMillis() < deadline, message);
    Sleep(1);
  }
}
void sharedAndPersistence(const std::wstring &program,
                          const std::wstring &dictionary) {
  Directory directory;
  const auto home = directory.path / L"共有 辞書";
  Broker broker(home, program, dictionary);
  auto a = broker.connect(), b = broker.connect();
  require(a->current.epoch == b->current.epoch, "clients must share broker");
  type(*a, "Kanji ");
  require(first(*a) == "漢字", "initial dictionary order");
  require(b->current.preedit.empty(), "preedit leaked to another client");
  a->perform(Op::Select, 1);
  require(a->current.committed == "感じ", "candidate choose");
  type(*b, "Kanji ");
  require(first(*b) == "感じ", "learned order not shared");
  b->perform(Op::Reset);
  type(*b, "Kaku");
  require(b->current.preedit == "▽かく", "direct kana");
  b->perform(Op::Reset);
  type(*b, "KaKu");
  require(b->current.preedit.find("書") != std::string::npos,
          "okuri conversion");
  key(*b, TGSKK_KEY_RETURN);
  require(b->current.committed == "書く", "okuri commit");
  type(*b, "Mihon ");
  require(b->current.preedit == "▼みほん【】", "missing word registration");
  type(*b, "mihonn");
  key(*b, TGSKK_KEY_RETURN);
  require(b->current.committed == "みほん", "new word commit");
  type(*a, "Mihon ");
  require(first(*a) == "みほん", "new word sharing");
  a->perform(Op::Reset);
  flush(*a);
  const auto saved = read(home / L"SKK-JISYO.user");
  require(saved.find("かんじ /感じ;感覚/") != std::string::npos,
          "learned dictionary missing");
  require(saved.find("みほん /みほん/") != std::string::npos,
          "registration not saved");
  Broker readonly(
      directory.path / L"readonly", program, dictionary,
      "UserDictionary=" + utf8((home / L"SKK-JISYO.user").wstring()) + "\n");
  auto ro = readonly.connect();
  require(ro->current.persistence == 3, "second writer not read-only");
  type(*ro, "Kanji ");
  require(first(*ro) == "感じ", "read-only user dictionary precedence");
  ro->perform(Op::Select, 1);
  require(read(home / L"SKK-JISYO.user") == saved,
          "second writer changed disk");
  readonly.stop();
  ro.reset();
  // Failure at .BAK must leave the live file intact and allow retry.
  const auto backup = home / L"SKK-JISYO.user.BAK";
  std::filesystem::remove(backup);
  std::filesystem::create_directory(backup);
  type(*a, "Kisha  ");
  key(*a, TGSKK_KEY_RETURN);
  waitUntil([&] { return a->perform(Op::Flush).persistence == 3; },
            "save failure not reported");
  require(read(home / L"SKK-JISYO.user") == saved,
          "failed save corrupted dictionary");
  type(*b, "Nihon ");
  require(first(*b) == "日本", "input blocked by failed save");
  b->perform(Op::Reset);
  std::filesystem::remove(backup);
  flush(*a);
  require(read(home / L"SKK-JISYO.user").find("きしゃ /汽車/") !=
              std::string::npos,
          "save did not recover");
  broker.stop();
  bool disconnected = false;
  try {
    key(*a, 'a');
  } catch (...) {
    disconnected = true;
  }
  require(disconnected, "dead broker accepted key");
  bool sticky = false;
  try {
    a->perform(Op::Status);
  } catch (...) {
    sticky = true;
  }
  require(sticky, "failed connection reused");
  a.reset();
  b.reset();
  Broker restarted(home, program, dictionary);
  auto c = restarted.connect();
  type(*c, "Kisha ");
  require(first(*c) == "汽車", "learning lost after restart");
  std::puts("shared sessions, okuri, registration, exclusive writer, atomic "
            "save failure/retry and restart passed");
}
void malformedAndIsolation(const std::wstring &program,
                           const std::wstring &dictionary) {
  Directory directory;
  Broker broker(directory.path, program, dictionary);
  auto a = broker.connect(), b = broker.connect();
  type(*a, "Kanji ");
  type(*b, "Kisha ");
  require(first(*a) == "漢字" && first(*b) == "記者", "session contamination");
  ++a->current.generation;
  bool stale = false;
  try {
    a->perform(Op::Select, 0);
  } catch (...) {
    stale = true;
  }
  require(stale, "stale candidate accepted");
  b->perform(Op::Reset);
  for (unsigned variant = 0; variant < 5; ++variant) {
    auto bad = broker.connect();
    Request q;
    q.sequence = 2;
    q.deadline = monotonicMillis() + 1000;
    q.fingerprint = broker.config.fingerprint;
    q.op = Op::Mode;
    q.argument = 256;
    auto body = encode(q);
    if (variant == 0)
      body[4] = 99;
    if (variant == 1)
      body.push_back(0);
    if (variant == 2) {
      body[body.size() - 4] = 1;
      body.push_back(0xff);
    }
    if (variant == 3)
      body.resize(3);
    Channel duplicate([&] {
      HANDLE copy = nullptr;
      require(DuplicateHandle(GetCurrentProcess(), bad->handle(),
                              GetCurrentProcess(), &copy, 0, FALSE,
                              DUPLICATE_SAME_ACCESS),
              "duplicate test pipe");
      return Handle(copy);
    }());
    require(duplicate.write(body, monotonicMillis() + 100),
            "send malformed frame");
    std::vector<uint8_t> reply;
    if (duplicate.read(reply, monotonicMillis() + 100))
      require(snapshot(reply).failed, "malformed request succeeded");
    type(*b, "a");
    require(b->current.committed == "あ",
            "malformed peer blocked valid client");
  }
  // An idle established connection must not own the engine thread.
  auto idle = broker.connect();
  type(*b, "Kanji ");
  require(first(*b) == "漢字", "idle peer blocked conversion");
  std::puts("stale generation, bad version/trailing/UTF-8/truncated/mode "
            "frames and idle-peer isolation passed");
}
class Server {
  SOCKET listener = INVALID_SOCKET;
  std::thread worker;
  std::atomic<bool> stopped = false;

public:
  std::atomic<unsigned> requests = 0;
  uint16_t port = 0;
  Server(std::string encoding, bool negative = false) {
    listener = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    require(listener != INVALID_SOCKET, "test server socket");
    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    require(bind(listener, reinterpret_cast<sockaddr *>(&address),
                 sizeof(address)) == 0 &&
                listen(listener, 4) == 0,
            "test server bind");
    int size = sizeof(address);
    require(getsockname(listener, reinterpret_cast<sockaddr *>(&address),
                        &size) == 0,
            "test server port");
    port = ntohs(address.sin_port);
    worker = std::thread([this, encoding, negative] {
      while (!stopped) {
        fd_set readSet;
        FD_ZERO(&readSet);
        FD_SET(listener, &readSet);
        timeval timeout{0, 100000};
        if (::select(0, &readSet, nullptr, nullptr, &timeout) <= 0)
          continue;
        SOCKET client = accept(listener, nullptr, nullptr);
        if (client == INVALID_SOCKET)
          continue;
        const DWORD receiveTimeout = 1000;
        setsockopt(client, SOL_SOCKET, SO_RCVTIMEO,
                   reinterpret_cast<const char *>(&receiveTimeout),
                   sizeof(receiveTimeout));
        while (!stopped) {
          char code;
          if (recv(client, &code, 1, 0) != 1 || code == '0')
            break;
          std::string reading;
          char c;
          while (recv(client, &c, 1, 0) == 1 && c != ' ')
            reading += c;
          ++requests;
          const auto expected = encoding == "utf-8"
                                    ? std::string("きしゃ")
                                    : std::string("\xa4\xad\xa4\xb7\xa4\xe3");
          const auto okuriKey = encoding == "utf-8" ? std::string("かk")
                                                    : std::string("\xa4\xabk");
          const bool okuri = reading == okuriKey;
          const bool valid = code == '1' && (reading == expected || okuri);
          Sleep(120);
          const std::string reply =
              negative || !valid ? "4\n"
              : encoding == "utf-8"
                  ? (okuri ? std::string("1/書/\n") : std::string("1/汽車/\n"))
              : okuri ? std::string("1/\xbd\xf1/\n")
                      : std::string("1/\xb5\xa5\xbc\xd6/\n");
          if (send(client, reply.data(), static_cast<int>(reply.size()), 0) !=
              static_cast<int>(reply.size()))
            break;
        }
        closesocket(client);
      }
    });
  }
  ~Server() {
    stopped = true;
    if (worker.joinable())
      worker.join();
    closesocket(listener);
  }
};
void serverCases(const std::wstring &program, const std::wstring &dictionary) {
  Directory directory;
  const auto empty = directory.path / L"empty";
  write(empty, ";; okuri-ari entries.\n;; okuri-nasi entries.\n");
  WSADATA sockets{};
  require(WSAStartup(MAKEWORD(2, 2), &sockets) == 0, "Winsock init");
  for (const std::string encoding : {"utf-8", "euc-jp"}) {
    Server server(encoding);
    Broker broker(directory.path / wide(encoding), program, dictionary,
                  "Skkserv=Custom\nServerEncoding=" + encoding +
                      "\nPort=" + std::to_string(server.port) + "\n");
    auto local = broker.connect();
    type(*local, "Kanji ", {}, 6);
    require(first(*local) == "漢字" && server.requests == 0,
            "local conversion accessed server");
    Broker missing(directory.path / (wide(encoding) + L"-missing"), program,
                   empty.wstring(),
                   "Skkserv=Custom\nServerEncoding=" + encoding +
                       "\nPort=" + std::to_string(server.port) + "\n");
    auto remote = missing.connect();
    type(*remote, "Kisha ", {}, 6);
    require((remote->current.flags & 16) &&
                remote->current.preedit == "▽きしゃ",
            "pending lookup did not preserve reading");
    waitUntil([&] { return server.requests == 1; },
              "server never received lookup");
    waitUntil(
        [&] {
          key(*remote, ' ', 0, {}, 6);
          return (remote->current.flags & 4) != 0;
        },
        "cached remote candidate unavailable");
    require(first(*remote) == "汽車", "wrong server encoding/result");
    remote->perform(Op::Reset);
    type(*remote, "Mihon ", {}, 4);
    require(!(remote->current.flags & 16), "network opt-out queued a lookup");
    require(server.requests == 1, "network opt-out sent data");
    remote->perform(Op::Reset);
    type(*remote, "KaKu", {}, 6);
    require(remote->current.flags & 16, "okuri lookup was not pending");
    waitUntil(
        [&] {
          key(*remote, ' ', 0, {}, 6);
          return (remote->current.flags & 4) != 0;
        },
        "late okuri lookup unavailable");
    require(remote->current.preedit == "▼書く",
            "okuri suffix or reading lost during pending rollback");

    key(*remote, 'g', TGSKK_MOD_CONTROL);
    remote.reset();
    local.reset();
    missing.stop();
    broker.stop();
  }
  // A confirmed miss must enter word entry rather than remain pending forever.
  {
    Server server("utf-8", true);
    Broker broker(directory.path / L"negative", program, empty.wstring(),
                  "Skkserv=Custom\nServerEncoding=utf-8\nPort=" +
                      std::to_string(server.port) + "\n");
    auto connection = broker.connect();
    type(*connection, "Kisha ", {}, 6);
    waitUntil(
        [&] {
          key(*connection, ' ', 0, {}, 6);
          return !(connection->current.flags & 16);
        },
        "cached miss remained pending");
    require(!(connection->current.flags & 4) &&
                connection->current.preedit != "▽きしゃ",
            "miss did not enter registration");
    connection->perform(Op::Reset);
    type(*connection, "Mihon ", {}, 6);
    key(*connection, ' ', TGSKK_MOD_SHIFT, {}, 6);
    require(!(connection->current.flags & 16) &&
                connection->current.preedit != "▽みほん",
            "Shift-Space did not force registration");
    connection.reset();
    broker.stop();
  }
  {
    const auto helper = (std::filesystem::path(program).parent_path() /
                         L"test-windows-child.exe")
                            .wstring();
    Broker broker(directory.path / L"google", program, empty.wstring(),
                  "Skkserv=Google\nGoogleHelper=" + utf8(helper) + "\n");
    auto connection = broker.connect();
    type(*connection, "Kisha ", {}, 6);
    waitUntil(
        [&] {
          key(*connection, ' ', 0, {}, 6);
          return (connection->current.flags & 4) != 0;
        },
        "Google helper lookup unavailable");
    require(first(*connection) == "汽車", "Google UTF-8 helper conversion");
    connection->perform(Op::Reset);
    type(*connection, "Kanji ", {}, 6);
    waitUntil(
        [&] {
          key(*connection, ' ', 0, {}, 6);
          return (connection->current.flags & 4) != 0;
        },
        "second Google helper lookup unavailable");
    require(first(*connection) == "漢字" &&
                connection->current.candidates[0].annotation == "文字",
            "Google helper annotation");
    connection.reset();
    broker.stop();
  }
  WSACleanup();
  std::puts("delayed custom UTF-8/EUC-JP, zero network on local hits/opt-out, "
            "cached misses and forced registration passed");
}
void adapterCases(const std::wstring &program, const std::wstring &dictionary) {
  Directory directory;
  Broker broker(directory.path, program, dictionary);
  auto ready = broker.connect();
  ready.reset();
  Handle blocker;
  waitUntil(
      [&] {
        blocker = openPipe(endpoint(Identity::current(), broker.config.home,
                                    broker.config.broker));
        return bool(blocker);
      },
      "block rendezvous");
  std::atomic<long> callbacks = 0;
  {
    Input input(broker.config, nullptr, &callbacks);
    // The non-acknowledging rendezvous peer keeps the connector from finishing.
    input.key('K', TGSKK_MOD_SHIFT);
    blocker = Handle();
    waitUntil([&] { return callbacks == 0; }, "connector did not complete");
    input.key('a', 0);
    require(!input.broker() && std::string(input.view()->preedit) == "▽か",
            "adopt replaced local composition");
    input.reset();
    input.key('q', 0);
    require(input.broker() && std::string(input.view()->mode_label) == "ア",
            "remote mode switch");
    broker.stop();
    require(input.key('a', 0) == nullptr && input.takeTransportFailure(),
            "transport failure not reported");
    require(std::string(input.view()->mode_label) == "ア",
            "fallback lost mode");
    require(input.key('a', 0) && std::string(input.view()->committed) == "ア",
            "local fallback input failed");
  }
  waitUntil([&] { return callbacks == 0; }, "connector outlived adapter");
  std::puts("deferred adoption, mode preservation, broker death and local "
            "fallback passed");
}

void lateBrokerRecovery(const std::wstring &program,
                        const std::wstring &dictionary) {
  Directory directory;
  write(directory.path / L"windows.ini",
        "Dictionary=" + utf8(dictionary) +
            "\nEncoding=utf-8\nAI=false\nSkkserv=Disabled\n");
  const auto delayedProgram = (directory.path / L"tgskk-broker.exe").wstring();
  const auto config = Config::load(directory.path.wstring(), delayedProgram);
  std::atomic<long> callbacks = 0;
  {
    Input input(config, nullptr, &callbacks);
    waitUntil([&] { return callbacks == 0; },
              "initial connector did not finish", 5000);
    require(input.key('a', 0) && !input.broker() &&
                std::string(input.view()->committed) == "あ",
            "initially unavailable broker blocked fallback");
    input.key('q', 0);
    std::filesystem::copy_file(program, delayedProgram);
    Broker broker(directory.path, delayedProgram, dictionary);
    waitUntil(
        [&] {
          const auto *view = input.key('a', 0);
          require(view && std::string(view->committed) == "ア",
                  "retry lost fallback key or mode");
          return input.broker();
        },
        "late broker was never adopted", 8000);
  }
  waitUntil([&] { return callbacks == 0; }, "late connector lifetime");
  std::puts("late broker availability retries without reactivation or mode "
            "loss passed");
}
void printLatency(std::vector<double> samples, const char *label) {
  require(!samples.empty(), "empty latency samples");
  std::sort(samples.begin(), samples.end());
  const auto n = samples.size();
  std::printf("%s us p50=%.1f p95=%.1f p99=%.1f max=%.1f (%zu keys)\n", label,
              samples[n / 2], samples[n * 95 / 100], samples[n * 99 / 100],
              samples.back(), n);
  require(samples[n * 99 / 100] < 10000,
          "input p99 exceeded the 10 ms performance target");
}
void performance(const std::wstring &program, const std::wstring &dictionary) {
  Directory directory;
  Broker broker(directory.path, program, dictionary);
  auto connection = broker.connect();
  auto idle = broker.connect();
  std::vector<double> samples;
  for (unsigned i = 0; i < 500; ++i) {
    const auto begin = std::chrono::steady_clock::now();
    key(*connection, 'a');
    samples.push_back(std::chrono::duration<double, std::micro>(
                          std::chrono::steady_clock::now() - begin)
                          .count());
    require(connection->current.committed == "あ", "fast typing lost key");
  }
  printLatency(samples, "broker kana RPC with idle peer");
  char *error = nullptr;
  auto *runtime =
      tgskk_runtime_new(utf8(dictionary).c_str(), "utf-8", nullptr, &error);
  require(runtime != nullptr, "baseline runtime");
  auto *session = tgskk_session_new(runtime, &error);
  require(session != nullptr, "baseline session");
  samples.clear();
  for (unsigned i = 0; i < 500; ++i) {
    const auto begin = std::chrono::steady_clock::now();
    const auto *view = tgskk_session_key(session, 'a', 0, 0);
    samples.push_back(std::chrono::duration<double, std::micro>(
                          std::chrono::steady_clock::now() - begin)
                          .count());
    require(view && std::string(view->committed) == "あ", "baseline key");
  }
  tgskk_session_free(session);
  tgskk_runtime_free(runtime);
  tgskk_string_free(error);
  printLatency(samples, "in-process C ABI kana baseline");
}
void tsfCase(const std::wstring &program, const std::wstring &dictionary,
             const std::wstring &harness, const std::wstring &dll,
             const std::wstring &eucDictionary, const std::wstring &worker = {},
             const std::wstring &model = {}) {
  Directory directory;
  Broker broker(directory.path, program, dictionary,
                worker.empty() ? ""
                               : "AI=true\nWorker=" + utf8(worker) +
                                     "\nModel=" + utf8(model) + "\n");
  auto observer = broker.connect();
  if (!worker.empty())
    waitUntil([&] { return observer->perform(Op::Status).ai == 1; },
              "TSF AI worker ready", 30000);
  const auto before = observer->current.processedKeys;
  SetEnvironmentVariableW(L"TGSKK_WINDOWS_HOME", directory.path.c_str());
  SetEnvironmentVariableW(L"TGSKK_TEST_EXTERNAL_REGISTRATION", L"1");
  auto command =
      L"\"" + harness + L"\" \"" + dll + L"\" \"" + eucDictionary + L"\"";
  if (!worker.empty())
    command += L" --ai";
  const auto logPath = directory.path / L"tsf-harness.log";
  auto process = startLogged(harness, command, logPath, 0);
  if (WaitForSingleObject(process.value, 90000) != WAIT_OBJECT_0) {
    TerminateProcess(process.value, 1);
    throw std::runtime_error("TSF broker test timeout");
  }
  DWORD result = 1;
  require(GetExitCodeProcess(process.value, &result), "TSF exit status");
  const auto log = read(logPath);
  std::printf("%s\n", log.c_str());
  require(result == 0, "TSF broker harness failed: " + std::to_string(result));
  if (!worker.empty())
    require(log.find("tgskk: edit request failed") == std::string::npos,
            "ordinary contextual commits reported failed edits");
  require(observer->perform(Op::Status).processedKeys > before + 10,
          "TSF test used only fallback instead of broker");
  flush(*observer);
  std::puts("native medium-integrity TSF used authenticated broker and "
            "released COM objects");
}
void aiCases(const std::wstring &program, const std::wstring &dictionary,
             const std::wstring &worker, const std::wstring &model) {
  Directory directory;
  Broker broker(directory.path, program, dictionary,
                "AI=true\nWorker=" + utf8(worker) + "\nModel=" + utf8(model) +
                    "\n");
  auto a = broker.connect(), b = broker.connect();
  waitUntil([&] { return a->perform(Op::Status).ai == 1; },
            "AI worker did not become ready", 30000);
  bool ranked = false;
  for (unsigned attempt = 0; attempt < 5 && !ranked; ++attempt) {
    a->perform(Op::Reset);
    type(*a, "Kisha", "駅で", 7, 50);
    const auto deadline = monotonicMillis() + 500;
    do {
      key(*a, ' ', 0, "駅で", 7);
      ranked = first(*a) == "汽車" && (a->current.flags & 8);
      if (ranked)
        break;
      a->perform(Op::Reset);
      type(*a, "Kisha", "駅で", 7, 50);
    } while (monotonicMillis() < deadline);
  }
  require(ranked, "contextual first candidate was never ready");
  a->perform(Op::Reset);
  type(*b, "Kisha ", {}, 4);
  require(first(*b) == "記者" && !(b->current.flags & 8),
          "AI/private-context state leaked between clients");
  b->perform(Op::Reset);
  type(*b, "Kisha", "新聞の", 7, 50);
  key(*b, ' ', 0, "新聞の", 7);
  require(first(*b) == "記者",
          "new foreground context retained train preference");
  b->perform(Op::Select, 0);
  type(*b, "Kisha     ", {}, 4);
  require(b->current.preedit == "▼きしゃ【】", "custom user word entry");
  type(*b, "mihonn", {}, 4);
  key(*b, TGSKK_KEY_RETURN);
  require(b->current.committed == "みほん", "custom word registration");
  type(*a, "Kisha", "駅で", 7, 50);
  key(*a, ' ', 0, "駅で", 7);
  require(first(*a) == "みほん" && !(a->current.flags & 8),
          "AI overrode protected user-only candidate");
  flush(*a);
  std::vector<double> samples;
  for (unsigned i = 0; i < 50; ++i) {
    a->perform(Op::Reset);
    for (unsigned char symbol : std::string("Kisha ")) {
      const auto begin = std::chrono::steady_clock::now();
      key(*a, symbol, symbol == 'K' ? TGSKK_MOD_SHIFT : 0,
          i % 2 ? "駅で" : "新聞の", 7);
      samples.push_back(std::chrono::duration<double, std::micro>(
                            std::chrono::steady_clock::now() - begin)
                            .count());
      Sleep(10);
    }
    require(a->current.flags & 4, "AI typing failed to convert");
  }
  printLatency(samples, "broker conversion keys with real AI worker");
  std::puts("real model context ranking, private context, foreground switching "
            "and user-only candidate protection passed");
}
} // namespace
int wmain(int argc, wchar_t **argv) {
  try {
    wchar_t log[32768];
    const auto length = GetEnvironmentVariableW(L"TGSKK_TEST_LOG", log, 32768);
    if (length) {
      require(length < 32768 && _wfreopen(log, L"wb", stdout),
              "open test output");
      require(_dup2(_fileno(stdout), _fileno(stderr)) == 0,
              "redirect test diagnostics");
      setvbuf(stdout, nullptr, _IONBF, 0);
    }
    require(argc == 3 || argc == 5 || argc == 7 || argc == 9,
            "Usage: test-broker broker dictionary [worker model | --tsf "
            "harness dll euc-dictionary]");
    Identity::current(); // Production authentication remains active in the
                         // test.
    const auto program = canonicalPath(argv[1]),
               dictionary = canonicalPath(argv[2]);
    if (argc == 9) {
      require(std::wstring(argv[3]) == L"--tsf-ai", "unknown broker test mode");
      tsfCase(program, dictionary, canonicalPath(argv[4]),
              canonicalPath(argv[5]), canonicalPath(argv[6]),
              canonicalPath(argv[7]), canonicalPath(argv[8]));
    } else if (argc == 7) {
      require(std::wstring(argv[3]) == L"--tsf", "unknown broker test mode");
      tsfCase(program, dictionary, canonicalPath(argv[4]),
              canonicalPath(argv[5]), canonicalPath(argv[6]));
    } else if (argc == 5)
      aiCases(program, dictionary, canonicalPath(argv[3]),
              canonicalPath(argv[4]));
    else {
      sharedAndPersistence(program, dictionary);
      malformedAndIsolation(program, dictionary);
      serverCases(program, dictionary);
      adapterCases(program, dictionary);
      lateBrokerRecovery(program, dictionary);
      performance(program, dictionary);
    }
    return 0;
  } catch (const std::exception &e) {
    std::fprintf(stderr, "test-broker: %s (Win32 %lu)\n", e.what(),
                 GetLastError());
    return 1;
  }
}
