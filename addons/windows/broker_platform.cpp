// SPDX-License-Identifier: GPL-3.0-or-later
#include "broker_platform.h"
#include <algorithm>
#include <bcrypt.h>
#include <fstream>
#include <map>
#include <sddl.h>
#include <shlobj.h>
#include <sstream>
#include <stdexcept>
#include <system_error>

namespace tgskk::windows {
static void require(bool okay, const char *stage) {
  if (!okay)
    throw std::runtime_error(std::string(stage) + " (" +
                             std::to_string(GetLastError()) + ")");
}
uint64_t monotonicMillis() {
  // QPC is shared across processes; GetTickCount64 can advance in 16 ms steps.
  static const uint64_t frequency = [] {
    LARGE_INTEGER n{};
    require(QueryPerformanceFrequency(&n) && n.QuadPart > 0,
            "Performance frequency");
    return static_cast<uint64_t>(n.QuadPart);
  }();
  LARGE_INTEGER n{};
  require(QueryPerformanceCounter(&n), "Performance counter");
  const auto ticks = static_cast<uint64_t>(n.QuadPart);
  return (ticks / frequency) * 1000 + (ticks % frequency) * 1000 / frequency;
}
std::string utf8(const std::wstring &text) {
  if (text.empty())
    return {};
  const int n = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                                    static_cast<int>(text.size()), nullptr, 0,
                                    nullptr, nullptr);
  require(n > 0, "UTF-16 conversion");
  std::string out(n, '\0');
  require(WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                              static_cast<int>(text.size()), out.data(), n,
                              nullptr, nullptr) == n,
          "UTF-16 conversion");
  return out;
}
std::wstring wide(const std::string &text) {
  if (text.empty())
    return {};
  require(text.find('\0') == std::string::npos, "Embedded NUL");
  const int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                    static_cast<int>(text.size()), nullptr, 0);
  require(n > 0, "UTF-8 conversion");
  std::wstring out(n, L'\0');
  require(MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                              static_cast<int>(text.size()), out.data(),
                              n) == n,
          "UTF-8 conversion");
  return out;
}
std::wstring executablePath(HMODULE module) {
  std::wstring path(32768, L'\0');
  const DWORD n =
      GetModuleFileNameW(module, path.data(), static_cast<DWORD>(path.size()));
  require(n && n < path.size(), "Module path");
  path.resize(n);
  return canonicalPath(path);
}
std::wstring canonicalPath(const std::wstring &path) {
  return std::filesystem::weakly_canonical(std::filesystem::absolute(path))
      .wstring();
}
static std::vector<uint8_t> tokenData(HANDLE token,
                                      TOKEN_INFORMATION_CLASS kind) {
  DWORD size = 0;
  GetTokenInformation(token, kind, nullptr, 0, &size);
  require(size && size <= 65536, "Token information size");
  std::vector<uint8_t> data(size);
  require(GetTokenInformation(token, kind, data.data(), size, &size),
          "Token information");
  return data;
}
Identity Identity::process(HANDLE process) {
  HANDLE raw = nullptr;
  require(OpenProcessToken(process, TOKEN_QUERY, &raw), "Process token");
  Handle token(raw);
  const auto integrity = tokenData(token.value, TokenIntegrityLevel);
  auto sid = reinterpret_cast<const TOKEN_MANDATORY_LABEL *>(integrity.data())
                 ->Label.Sid;
  const DWORD level =
      *GetSidSubAuthority(sid, *GetSidSubAuthorityCount(sid) - 1);
  require(level == SECURITY_MANDATORY_MEDIUM_RID,
          "Only medium integrity is supported");
  const auto elevated = tokenData(token.value, TokenElevation);
  require(!reinterpret_cast<const TOKEN_ELEVATION *>(elevated.data())
               ->TokenIsElevated,
          "Elevated token");
  const auto container = tokenData(token.value, TokenIsAppContainer);
  require(!*reinterpret_cast<const DWORD *>(container.data()),
          "AppContainer token");
  Identity identity;
  const auto session = tokenData(token.value, TokenSessionId);
  identity.session = *reinterpret_cast<const DWORD *>(session.data());
  const auto groups = tokenData(token.value, TokenGroups);
  const auto *items = reinterpret_cast<const TOKEN_GROUPS *>(groups.data());
  for (DWORD i = 0; i < items->GroupCount; ++i) {
    if ((items->Groups[i].Attributes & SE_GROUP_LOGON_ID) ==
        SE_GROUP_LOGON_ID) {
      LPWSTR text = nullptr;
      require(ConvertSidToStringSidW(items->Groups[i].Sid, &text), "Logon SID");
      identity.logon = text;
      LocalFree(text);
      break;
    }
  }
  require(!identity.logon.empty(), "Missing logon SID");
  return identity;
}
Identity Identity::current() { return process(GetCurrentProcess()); }
PipeSecurity::PipeSecurity(const Identity &identity) {
  // Unique first instances avoid granting FILE_CREATE_PIPE_INSTANCE to clients.
  const std::wstring sddl = L"D:P(A;;0x100083;;;" + identity.logon + L")";
  require(ConvertStringSecurityDescriptorToSecurityDescriptorW(
              sddl.c_str(), SDDL_REVISION_1, &descriptor, nullptr),
          "Pipe security");
  attributes = {sizeof(attributes), descriptor, FALSE};
}
PipeSecurity::~PipeSecurity() {
  if (descriptor)
    LocalFree(descriptor);
}
Handle makePipe(const std::wstring &name, PipeSecurity &security) {
  Handle pipe(CreateNamedPipeW(name.c_str(),
                               PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED |
                                   FILE_FLAG_FIRST_PIPE_INSTANCE,
                               PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT |
                                   PIPE_REJECT_REMOTE_CLIENTS,
                               1, 65536, 65536, 0, &security.attributes));
  require(bool(pipe), "Create named pipe");
  return pipe;
}
bool acceptPipe(HANDLE pipe, HANDLE stop, DWORD timeout) {
  Handle event(CreateEventW(nullptr, TRUE, FALSE, nullptr));
  if (!event)
    return false;
  const auto started = GetTickCount64();
  for (;;) {
    if (stop && WaitForSingleObject(stop, 0) == WAIT_OBJECT_0)
      return false;
    const auto elapsed = GetTickCount64() - started;
    if (timeout != INFINITE && elapsed >= timeout)
      return false;
    OVERLAPPED overlap{};
    overlap.hEvent = event.value;
    ResetEvent(event.value);
    if (ConnectNamedPipe(pipe, &overlap))
      return true;
    const DWORD error = GetLastError();
    if (error == ERROR_PIPE_CONNECTED)
      return true;
    if (error == ERROR_NO_DATA) {
      // A client can time out while the broker is still loading dictionaries.
      // Reset its abandoned connection instead of losing the listener thread.
      if (!DisconnectNamedPipe(pipe))
        return false;
      continue;
    }
    if (error != ERROR_IO_PENDING)
      return false;
    HANDLE events[] = {event.value, stop};
    const DWORD remaining =
        timeout == INFINITE ? INFINITE : timeout - static_cast<DWORD>(elapsed);
    const DWORD result =
        WaitForMultipleObjects(stop ? 2 : 1, events, FALSE, remaining);
    if (result != WAIT_OBJECT_0)
      CancelIoEx(pipe, &overlap);
    DWORD bytes = 0;
    const BOOL done = GetOverlappedResult(pipe, &overlap, &bytes, TRUE);
    return result == WAIT_OBJECT_0 && done;
  }
}
Channel::Channel(Handle handle)
    : pipe(std::move(handle)),
      event(CreateEventW(nullptr, TRUE, FALSE, nullptr)) {
  require(bool(pipe) && bool(event), "Pipe channel");
}
bool Channel::transfer(void *data, DWORD size, bool writing,
                       uint64_t deadline) {
  auto *bytes = static_cast<uint8_t *>(data);
  while (size) {
    if (deadline != UINT64_MAX && monotonicMillis() >= deadline)
      return false;
    OVERLAPPED overlap{};
    overlap.hEvent = event.value;
    ResetEvent(event.value);
    DWORD count = 0;
    BOOL okay = writing ? WriteFile(pipe.value, bytes, size, &count, &overlap)
                        : ReadFile(pipe.value, bytes, size, &count, &overlap);
    if (!okay) {
      if (GetLastError() != ERROR_IO_PENDING)
        return false;
      const auto now = monotonicMillis();
      const DWORD wait =
          deadline == UINT64_MAX
              ? INFINITE
              : static_cast<DWORD>(std::min<uint64_t>(
                    deadline > now ? deadline - now : 0, MAXDWORD - 1));
      if (WaitForSingleObject(event.value, wait) != WAIT_OBJECT_0) {
        CancelIoEx(pipe.value, &overlap);
        // Cancellation is asynchronous: the stack buffer must remain live.
        GetOverlappedResult(pipe.value, &overlap, &count, TRUE);
        return false;
      }
      okay = GetOverlappedResult(pipe.value, &overlap, &count, FALSE);
    }
    if (!okay || !count || count > size)
      return false;
    bytes += count;
    size -= count;
  }
  return true;
}
bool Channel::write(const std::vector<uint8_t> &body, uint64_t deadline) {
  if (body.size() > 65536)
    return false;
  std::vector<uint8_t> frame;
  for (unsigned shift = 0; shift < 32; shift += 8)
    frame.push_back(static_cast<uint8_t>(body.size() >> shift));
  frame.insert(frame.end(), body.begin(), body.end());
  return transfer(frame.data(), static_cast<DWORD>(frame.size()), true,
                  deadline);
}
bool Channel::read(std::vector<uint8_t> &body, uint64_t deadline) {
  uint8_t header[4];
  if (!transfer(header, 4, false, deadline))
    return false;
  uint32_t size = 0;
  for (unsigned i = 0; i < 4; ++i)
    size |= uint32_t(header[i]) << (i * 8);
  if (!size || size > 65536)
    return false;
  body.resize(size);
  return transfer(body.data(), size, false, deadline);
}
Handle openPipe(const std::wstring &name) {
  return Handle(CreateFileW(
      name.c_str(),
      FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE, 0,
      nullptr, OPEN_EXISTING,
      FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
      nullptr));
}
void verifyPeer(HANDLE pipe, bool server, const Identity &identity,
                const std::wstring &expected) {
  ULONG pid = 0;
  require(server ? GetNamedPipeClientProcessId(pipe, &pid)
                 : GetNamedPipeServerProcessId(pipe, &pid),
          "Pipe peer PID");
  Handle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid));
  require(bool(process), "Pipe peer process");
  require(Identity::process(process.value) == identity, "Pipe peer logon");
  if (!expected.empty()) {
    std::wstring path(32768, L'\0');
    DWORD n = static_cast<DWORD>(path.size());
    require(QueryFullProcessImageNameW(process.value, 0, path.data(), &n),
            "Broker image path");
    path.resize(n);
    const auto actual = canonicalPath(path), wanted = canonicalPath(expected);
    require(CompareStringOrdinal(actual.c_str(), -1, wanted.c_str(), -1,
                                 TRUE) == CSTR_EQUAL,
            "Unexpected broker image");
  }
}
uint64_t randomId() {
  uint64_t value = 0;
  require(BCryptGenRandom(nullptr, reinterpret_cast<PUCHAR>(&value),
                          sizeof(value),
                          BCRYPT_USE_SYSTEM_PREFERRED_RNG) == 0 &&
              value,
          "Random identifier");
  return value;
}
static uint64_t hash(const std::string &text) {
  uint64_t value = 14695981039346656037ULL;
  for (unsigned char c : text) {
    value ^= c;
    value *= 1099511628211ULL;
  }
  return value;
}
std::wstring endpoint(const Identity &identity, const std::wstring &home,
                      const std::wstring &broker) {
  return L"\\\\.\\pipe\\tgskk-v1-" + identity.logon + L"-" +
         std::to_wstring(hash(utf8(canonicalPath(home)) + "|" +
                              utf8(canonicalPath(broker))));
}
std::wstring homeDirectory() {
  wchar_t custom[32768];
  const DWORD n = GetEnvironmentVariableW(L"TGSKK_WINDOWS_HOME", custom, 32768);
  require(n < 32768, "Configuration directory length");
  if (n) {
    require(std::filesystem::path(custom).is_absolute(),
            "Configuration directory must be absolute");
    return canonicalPath(custom);
  }
  PWSTR path = nullptr;
  require(
      SUCCEEDED(SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr, &path)),
      "LocalAppData");
  std::wstring home = (std::filesystem::path(path) / L"tgskk").wstring();
  CoTaskMemFree(path);
  return canonicalPath(home);
}
static Config parseConfig(const std::wstring &directory,
                          const std::wstring &program, std::istream &input) {
  Config c;
  c.home = canonicalPath(directory);
  c.broker = canonicalPath(program);
  const auto sibling = std::filesystem::path(c.broker).parent_path();
  const auto defaults = sibling / L"windows-defaults.ini";
  if (GetPrivateProfileIntW(L"General", L"GoogleLookup", 1, defaults.c_str()) ==
      0)
    c.server = "Disabled";
  c.dictionary = (sibling / L"SKK-JISYO.L").wstring();
  c.userDictionary =
      (std::filesystem::path(c.home) / L"SKK-JISYO.user").wstring();
  c.worker = (sibling / L"tgskk-zenz-worker.exe").wstring();
  c.helper = (sibling / L"tgskk-google-skkserv.exe").wstring();
  c.model = (std::filesystem::path(c.home) / L"models" /
             L"zenz-v3.2-small-Q5_K_M.gguf")
                .wstring();
  wchar_t dictionary[32768];
  const auto n =
      GetEnvironmentVariableW(L"TGSKK_DICTIONARY", dictionary, 32768);
  require(n < 32768, "Dictionary path length");
  if (n)
    c.dictionary = canonicalPath(dictionary);
  std::string line, configuration;
  auto trim = [](std::string value) {
    const auto first = value.find_first_not_of(" \t\r\n");
    if (first == std::string::npos)
      return std::string();
    return value.substr(first, value.find_last_not_of(" \t\r\n") - first + 1);
  };
  auto path = [&](const std::string &value) {
    auto p = std::filesystem::path(wide(value));
    return canonicalPath(
        (p.is_absolute() ? p : std::filesystem::path(c.home) / p).wstring());
  };
  size_t length = 0;
  bool firstLine = true, explicitModel = false;
  while (std::getline(input, line)) {
    length += line.size() + 1;
    if (length > 65536)
      throw std::runtime_error("windows.ini is too large");
    if (firstLine && line.starts_with("\xef\xbb\xbf"))
      line.erase(0, 3);
    firstLine = false;
    line = trim(line);
    if (line.empty() || line[0] == '#' || line[0] == ';' || line == "[General]")
      continue;
    const auto separator = line.find('=');
    if (separator == std::string::npos)
      throw std::runtime_error("Invalid windows.ini entry");
    const auto key = trim(line.substr(0, separator)),
               value = trim(line.substr(separator + 1));
    configuration += key + "=" + value + "\n";
    if (key == "Dictionary")
      c.dictionary = path(value);
    else if (key == "Encoding")
      c.encoding = value;
    else if (key == "UserDictionary")
      c.userDictionary = path(value);
    else if (key == "Worker")
      c.worker = path(value);
    else if (key == "Model") {
      c.model = path(value);
      explicitModel = true;
    } else if (key == "GoogleHelper")
      c.helper = path(value);
    else if (key == "AI") {
      if (value != "true" && value != "false")
        throw std::runtime_error("AI must be true or false");
      c.ai = value == "true";
    } else if (key == "SkkservWithoutInputScope") {
      if (value != "true" && value != "false")
        throw std::runtime_error(
            "SkkservWithoutInputScope must be true or false");
      c.skkservWithoutInputScope = value == "true";
    } else if (key == "Skkserv")
      c.server = value;
    else if (key == "Host")
      c.host = value;
    else if (key == "ServerEncoding")
      c.serverEncoding = value;
    else if (key == "InlineCandidates") {
      size_t used = 0;
      const auto count = std::stoul(value, &used);
      if (count > 64 || used != value.size())
        throw std::runtime_error("InlineCandidates must be 0..64");
      c.inlineCandidates = static_cast<uint32_t>(count);
    } else if (key == "Port") {
      size_t used = 0;
      unsigned long port = std::stoul(value, &used);
      if (!port || port > 65535 || used != value.size())
        throw std::runtime_error("Invalid SKK port");
      c.port = static_cast<uint16_t>(port);
    } else
      throw std::runtime_error("Unknown windows.ini key: " + key);
  }
  if (c.encoding != "euc-jp" && c.encoding != "utf-8")
    throw std::runtime_error("Invalid dictionary encoding");
  if (c.serverEncoding != "euc-jp" && c.serverEncoding != "utf-8")
    throw std::runtime_error("Invalid SKK server encoding");
  if (c.server != "Google" && c.server != "Custom" && c.server != "Disabled")
    throw std::runtime_error("Invalid SKK server mode");
  if (!explicitModel) {
    std::error_code error;
    const auto userModel = std::filesystem::status(c.model, error);
    if (userModel.type() == std::filesystem::file_type::not_found) {
      const auto shared = sibling / L"models" / L"zenz-v3.2-small-Q5_K_M.gguf";
      if (std::filesystem::is_regular_file(shared, error))
        c.model = shared.wstring();
    }
  }
  c.fingerprint = hash(configuration + c.server + "\n" + utf8(c.dictionary) +
                       utf8(c.worker) + utf8(c.model) + utf8(c.helper) +
                       utf8(c.userDictionary));
  return c;
}
Config Config::load(const std::wstring &directory,
                    const std::wstring &program) {
  const auto ini = std::filesystem::path(directory) / L"windows.ini";
  std::ifstream input(ini, std::ios::binary);
  if (!input && std::filesystem::exists(ini))
    throw std::runtime_error("Cannot read windows.ini");
  return parseConfig(directory, program, input);
}
void Config::save(const Config &config) {
  std::ostringstream text;
  text << "[General]\n";
  const auto field = [&](const char *key, const std::string &value) {
    if (value.find_first_of("\r\n") != std::string::npos ||
        value.find('\0') != std::string::npos)
      throw std::runtime_error(
          "Configuration values cannot contain newlines or NULs");
    text << key << '=' << value << '\n';
  };
  const std::pair<const char *, std::wstring> paths[] = {
      {"Dictionary", config.dictionary},
      {"UserDictionary", config.userDictionary},
      {"Worker", config.worker},
      {"Model", config.model},
      {"GoogleHelper", config.helper}};
  for (const auto &entry : paths) {
    if (entry.second.empty())
      throw std::runtime_error("A file path cannot be empty");
    field(entry.first, utf8(entry.second));
  }
  field("Encoding", config.encoding);
  field("AI", config.ai ? "true" : "false");
  field("Skkserv", config.server);
  field("SkkservWithoutInputScope",
        config.skkservWithoutInputScope ? "true" : "false");
  field("Host", config.host);
  field("Port", std::to_string(config.port));
  field("ServerEncoding", config.serverEncoding);
  field("InlineCandidates", std::to_string(config.inlineCandidates));
  if (config.server == "Custom" && config.host.empty())
    throw std::runtime_error("Custom server host is empty");
  const auto bytes = text.str();
  std::istringstream validation(bytes);
  (void)parseConfig(config.home, config.broker, validation);
  const auto directory = std::filesystem::path(config.home);
  std::filesystem::create_directories(directory);
  const auto temporary =
      directory / (L"windows.ini." + std::to_wstring(randomId()) + L".tmp");
  try {
    {
      Handle output(CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr,
                                CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr));
      require(bool(output), "Create temporary settings");
      DWORD written = 0;
      require(WriteFile(output.value, bytes.data(),
                        static_cast<DWORD>(bytes.size()), &written, nullptr) &&
                  written == bytes.size(),
              "Write settings");
      require(FlushFileBuffers(output.value), "Flush settings");
    }
    require(MoveFileExW(temporary.c_str(), (directory / L"windows.ini").c_str(),
                        MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH),
            "Replace settings");
  } catch (...) {
    std::error_code ignored;
    std::filesystem::remove(temporary, ignored);
    throw;
  }
}
static std::wstring quote(const std::wstring &value) {
  std::wstring out = L"\"";
  size_t slashes = 0;
  for (wchar_t c : value) {
    if (c == L'\\') {
      ++slashes;
      continue;
    }
    if (c == L'\"')
      out.append(slashes * 2 + 1, L'\\');
    else
      out.append(slashes, L'\\');
    slashes = 0;
    out += c;
  }
  out.append(slashes * 2, L'\\');
  return out + L"\"";
}
bool launchBroker(const Config &config) {
  BOOL inJob = FALSE;
  if (!IsProcessInJob(GetCurrentProcess(), nullptr, &inJob))
    return false;
  auto command = quote(config.broker) + L" --home " + quote(config.home);
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(
          config.broker.c_str(), command.data(), nullptr, nullptr, FALSE,
          CREATE_NO_WINDOW | (inJob ? CREATE_BREAKAWAY_FROM_JOB : 0), nullptr,
          std::filesystem::path(config.broker).parent_path().c_str(), &startup,
          &process))
    return false;
  CloseHandle(process.hThread);
  CloseHandle(process.hProcess);
  return true;
}
} // namespace tgskk::windows
