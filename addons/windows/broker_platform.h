// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <filesystem>
#include <memory>
#include <string>
#include <vector>
#include <windows.h>

namespace tgskk::windows {
struct Handle {
  HANDLE value = INVALID_HANDLE_VALUE;
  explicit Handle(HANDLE h = INVALID_HANDLE_VALUE) : value(h) {}
  ~Handle() {
    if (value && value != INVALID_HANDLE_VALUE)
      CloseHandle(value);
  }
  Handle(const Handle &) = delete;
  Handle &operator=(const Handle &) = delete;
  Handle(Handle &&other) noexcept : value(other.value) {
    other.value = INVALID_HANDLE_VALUE;
  }
  Handle &operator=(Handle &&other) noexcept {
    std::swap(value, other.value);
    return *this;
  }
  explicit operator bool() const {
    return value && value != INVALID_HANDLE_VALUE;
  }
};
std::string utf8(const std::wstring &);
std::wstring wide(const std::string &);
std::wstring executablePath(HMODULE = nullptr);
std::wstring canonicalPath(const std::wstring &);
struct Identity {
  std::wstring logon;
  DWORD session = 0;
  static Identity process(HANDLE);
  static Identity current();
  bool operator==(const Identity &) const = default;
};
class PipeSecurity {
  PSECURITY_DESCRIPTOR descriptor = nullptr;

public:
  SECURITY_ATTRIBUTES attributes{};
  explicit PipeSecurity(const Identity &);
  ~PipeSecurity();
  PipeSecurity(const PipeSecurity &) = delete;
};
Handle makePipe(const std::wstring &, PipeSecurity &);
bool acceptPipe(HANDLE, HANDLE stop = nullptr, DWORD timeout = INFINITE);
class Channel {
  Handle pipe, event;

public:
  explicit Channel(Handle);
  HANDLE handle() const { return pipe.value; }
  bool read(std::vector<uint8_t> &, uint64_t deadline);
  bool write(const std::vector<uint8_t> &, uint64_t deadline);

private:
  bool transfer(void *, DWORD, bool, uint64_t);
};
Handle openPipe(const std::wstring &);
void verifyPeer(HANDLE, bool server, const Identity &,
                const std::wstring &expectedExecutable = {});
uint64_t randomId();
uint64_t monotonicMillis();
std::wstring endpoint(const Identity &, const std::wstring &home,
                      const std::wstring &broker);
std::wstring homeDirectory();
struct Config {
  std::wstring home, broker, dictionary, userDictionary, worker, model, helper;
  std::string encoding = "euc-jp", server = "Google", host = "127.0.0.1",
              serverEncoding = "euc-jp";
  uint16_t port = 1178;
  uint32_t inlineCandidates = 4;
  bool ai = true;
  bool skkservWithoutInputScope = false;
  uint64_t fingerprint = 0;
  static Config load(const std::wstring &home, const std::wstring &broker);
  static void save(const Config &);
};
bool launchBroker(const Config &);
} // namespace tgskk::windows
