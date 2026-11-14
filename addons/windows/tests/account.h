// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "../broker_platform.h"
#include <aclapi.h>
#include <algorithm>
#include <cstdio>
#include <cwchar>
#include <lm.h>
#include <sddl.h>
#include <stdexcept>
#include <userenv.h>
namespace tgskk::windows::test {
inline void checked(bool okay, const char *stage) {
  if (!okay)
    throw std::runtime_error(std::string(stage) + ": " +
                             std::to_string(GetLastError()));
}
class FileAccess {
  std::wstring path;
  PSECURITY_DESCRIPTOR previous = nullptr;
  PACL oldAcl = nullptr;

public:
  FileAccess(const std::wstring &name, PSID user, DWORD access, DWORD inherit,
             ACCESS_MODE mode = GRANT_ACCESS)
      : path(name) {
    checked(GetNamedSecurityInfoW(path.data(), SE_FILE_OBJECT,
                                  DACL_SECURITY_INFORMATION, nullptr, nullptr,
                                  &oldAcl, nullptr, &previous) == ERROR_SUCCESS,
            "Read test file security");
    EXPLICIT_ACCESSW entry{};
    entry.grfAccessPermissions = access;
    entry.grfAccessMode = mode;
    entry.grfInheritance = inherit;
    entry.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    entry.Trustee.ptstrName = static_cast<LPWSTR>(user);
    PACL updated = nullptr;
    const auto merged = SetEntriesInAclW(1, &entry, oldAcl, &updated);
    const auto changed =
        merged == ERROR_SUCCESS
            ? SetNamedSecurityInfoW(path.data(), SE_FILE_OBJECT,
                                    DACL_SECURITY_INFORMATION, nullptr, nullptr,
                                    updated, nullptr)
            : merged;
    if (updated)
      LocalFree(updated);
    if (changed != ERROR_SUCCESS) {
      LocalFree(previous);
      previous = nullptr;
      SetLastError(changed);
      checked(false, "Set test file access");
    }
  }
  bool close() noexcept {
    if (!previous)
      return true;
    const auto restored = SetNamedSecurityInfoW(
        path.data(), SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, nullptr,
        nullptr, oldAcl, nullptr);
    if (restored != ERROR_SUCCESS) {
      std::fprintf(stderr, "test-user: ACL rollback failed: %lu\n", restored);
      return false;
    }
    LocalFree(previous);
    previous = nullptr;
    return true;
  }
  ~FileAccess() {
    if (!close() && previous)
      LocalFree(previous);
  }
};
class Account {
  bool owned = false;
  PROFILEINFOW profile{};
  bool cleanup() noexcept {
    bool okay = true;
    const auto check = [&](bool success, DWORD code, const char *stage) {
      if (!success) {
        std::fprintf(stderr, "test-user: %s failed: %lu\n", stage, code);
        okay = false;
      }
    };
    if (!password.empty())
      SecureZeroMemory(password.data(), password.size() * sizeof(wchar_t));
    if (environment) {
      const bool destroyed = DestroyEnvironmentBlock(environment);
      check(destroyed, GetLastError(), "environment cleanup");
      environment = nullptr;
    }
    if (profile.hProfile) {
      const bool unloaded = UnloadUserProfile(token.value, profile.hProfile);
      check(unloaded, GetLastError(), "profile unload");
      if (unloaded)
        profile.hProfile = nullptr;
    }
    if (owned) {
      const auto removed = NetUserDel(nullptr, name.c_str());
      check(removed == NERR_Success, removed, "account removal");
      if (removed == NERR_Success)
        owned = false;
    }
    if (!sidText.empty() && !profile.hProfile) {
      const bool deleted = DeleteProfileW(sidText.c_str(), nullptr, nullptr);
      const auto code = GetLastError();
      check(deleted || code == ERROR_FILE_NOT_FOUND, code, "profile deletion");
      if (deleted || code == ERROR_FILE_NOT_FOUND)
        sidText.clear();
    }
    return okay;
  }

public:
  std::wstring name, password, sidText;
  Handle token;
  void *environment = nullptr;
  Account(HANDLE administrator) {
    wchar_t ci[8];
    checked(GetEnvironmentVariableW(L"GITHUB_ACTIONS", ci, 8) == 4 &&
                std::wstring(ci, 4) == L"true",
            "Disposable CI is required for test account creation");
    name = L"tgskk-" + std::to_wstring(randomId()).substr(0, 12);
    password =
        L"Tg!7" + std::to_wstring(randomId()) + std::to_wstring(randomId());
    try {
      USER_INFO_1 user{};
      user.usri1_name = name.data();
      user.usri1_password = password.data();
      user.usri1_priv = USER_PRIV_USER;
      user.usri1_flags = UF_SCRIPT | UF_NORMAL_ACCOUNT | UF_DONT_EXPIRE_PASSWD;
      DWORD parameter = 0;
      const auto added =
          NetUserAdd(nullptr, 1, reinterpret_cast<BYTE *>(&user), &parameter);
      if (added != NERR_Success) {
        SetLastError(added);
        checked(false, "Create disposable standard user");
      }
      owned = true;
      HANDLE handle = nullptr;
      checked(LogonUserW(name.c_str(), L".", password.c_str(),
                         LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT,
                         &handle),
              "Log on disposable user");
      token = Handle(handle);
      DWORD needed = 0;
      GetTokenInformation(token.value, TokenUser, nullptr, 0, &needed);
      std::vector<uint8_t> data(needed);
      checked(GetTokenInformation(token.value, TokenUser, data.data(), needed,
                                  &needed),
              "Standard user SID");
      LPWSTR text = nullptr;
      checked(ConvertSidToStringSidW(
                  reinterpret_cast<TOKEN_USER *>(data.data())->User.Sid, &text),
              "Format user SID");
      sidText = text;
      LocalFree(text);
      for (auto privilege : {SE_BACKUP_NAME, SE_RESTORE_NAME}) {
        TOKEN_PRIVILEGES desired{};
        desired.PrivilegeCount = 1;
        checked(LookupPrivilegeValueW(nullptr, privilege,
                                      &desired.Privileges[0].Luid),
                "Profile privilege");
        desired.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        SetLastError(ERROR_SUCCESS);
        checked(AdjustTokenPrivileges(administrator, FALSE, &desired, 0,
                                      nullptr, nullptr) &&
                    GetLastError() == ERROR_SUCCESS,
                "Enable test profile privilege");
      }
      profile.dwSize = sizeof(profile);
      profile.dwFlags = PI_NOUI;
      profile.lpUserName = name.data();
      checked(LoadUserProfileW(token.value, &profile),
              "Load disposable profile");
      checked(CreateEnvironmentBlock(&environment, token.value, FALSE),
              "Standard user environment");
    } catch (...) {
      cleanup();
      throw;
    }
  }
  bool close() noexcept { return cleanup(); }
  ~Account() { cleanup(); }
  std::vector<wchar_t> childEnvironment(const std::wstring &log) const {
    std::vector<std::wstring> entries;
    const auto *item = static_cast<const wchar_t *>(environment);
    while (*item) {
      const size_t length = std::wcslen(item) + 1;
      if (_wcsnicmp(item, L"TGSKK_TEST_LOG=",
                    std::wcslen(L"TGSKK_TEST_LOG=")) &&
          _wcsnicmp(item, L"TGSKK_DEBUG=", std::wcslen(L"TGSKK_DEBUG=")))
        entries.emplace_back(item);
      item += length;
    }
    entries.push_back(L"TGSKK_TEST_LOG=" + log);
    entries.push_back(L"TGSKK_DEBUG=1");
    std::sort(entries.begin(), entries.end(), [](const auto &a, const auto &b) {
      return CompareStringOrdinal(a.c_str(), -1, b.c_str(), -1, TRUE) ==
             CSTR_LESS_THAN;
    });
    std::vector<wchar_t> result;
    for (const auto &entry : entries) {
      result.insert(result.end(), entry.begin(), entry.end());
      result.push_back(0);
    }
    result.push_back(0);
    return result;
  }
};
} // namespace tgskk::windows::test
