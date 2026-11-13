// SPDX-License-Identifier: GPL-3.0-or-later
#include "../broker_platform.h"
#include "account.h"
#include <aclapi.h>
#include <cstdio>
#include <fstream>
#include <sddl.h>
#include <stdexcept>
using namespace tgskk::windows;
static void require(bool okay, const char *stage) {
  if (!okay)
    throw std::runtime_error(std::string(stage) + ": " +
                             std::to_string(GetLastError()));
}
static std::wstring quote(const std::wstring &s) {
  std::wstring out = L"\"";
  size_t slash = 0;
  for (auto c : s) {
    if (c == '\\') {
      ++slash;
      continue;
    }
    out.append(slash * (c == '"' ? 2 : 1) + (c == '"' ? 1 : 0), L'\\');
    slash = 0;
    out += c;
  }
  out.append(slash * 2, L'\\');
  return out + L"\"";
}
static std::vector<uint8_t> tokenInformation(HANDLE token,
                                             TOKEN_INFORMATION_CLASS kind) {
  DWORD length = 0;
  GetTokenInformation(token, kind, nullptr, 0, &length);
  require(length > 0, "Token information size");
  std::vector<uint8_t> data(length);
  require(GetTokenInformation(token, kind, data.data(), length, &length),
          "Token information");
  return data;
}
class DesktopAccess {
  HANDLE object;
  std::vector<uint8_t> previous;

public:
  DesktopAccess(HANDLE value, PSID logon, DWORD access) : object(value) {
    SECURITY_INFORMATION info = DACL_SECURITY_INFORMATION;
    DWORD length = 0;
    GetUserObjectSecurity(object, &info, nullptr, 0, &length);
    require(length > 0, "Desktop security size");
    previous.resize(length);
    require(
        GetUserObjectSecurity(object, &info, previous.data(), length, &length),
        "Desktop security");
    PACL acl = nullptr, updated = nullptr;
    BOOL present = FALSE, defaulted = FALSE;
    require(GetSecurityDescriptorDacl(previous.data(), &present, &acl,
                                      &defaulted) &&
                present && acl,
            "Desktop DACL");
    EXPLICIT_ACCESSW entry{};
    entry.grfAccessPermissions = access;
    entry.grfAccessMode = GRANT_ACCESS;
    entry.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    entry.Trustee.ptstrName = static_cast<LPWSTR>(logon);
    require(SetEntriesInAclW(1, &entry, acl, &updated) == ERROR_SUCCESS,
            "Grant test desktop access");
    SECURITY_DESCRIPTOR descriptor{};
    const bool ready =
        InitializeSecurityDescriptor(&descriptor,
                                     SECURITY_DESCRIPTOR_REVISION) &&
        SetSecurityDescriptorDacl(&descriptor, TRUE, updated, FALSE) &&
        SetUserObjectSecurity(object, &info, &descriptor);
    LocalFree(updated);
    require(ready, "Set test desktop access");
  }
  bool close() noexcept {
    if (previous.empty())
      return true;
    SECURITY_INFORMATION info = DACL_SECURITY_INFORMATION;
    if (!SetUserObjectSecurity(object, &info, previous.data())) {
      std::fprintf(stderr, "test-user: desktop ACL rollback failed: %lu\n",
                   GetLastError());
      return false;
    }
    previous.clear();
    return true;
  }
  ~DesktopAccess() { close(); }
};
class ProcessTree {
public:
  Handle handle{CreateJobObjectW(nullptr, nullptr)};
  ProcessTree() {
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    require(handle && SetInformationJobObject(handle.value,
                                              JobObjectExtendedLimitInformation,
                                              &limits, sizeof(limits)),
            "Test process containment");
  }
  bool close() noexcept {
    if (!handle)
      return true;
    if (!TerminateJobObject(handle.value, 0)) {
      std::fprintf(stderr, "test-user: descendant termination failed: %lu\n",
                   GetLastError());
      return false;
    }
    const auto deadline = GetTickCount64() + 10000;
    for (;;) {
      JOBOBJECT_BASIC_ACCOUNTING_INFORMATION accounting{};
      if (!QueryInformationJobObject(
              handle.value, JobObjectBasicAccountingInformation, &accounting,
              sizeof(accounting), nullptr)) {
        std::fprintf(stderr, "test-user: descendant accounting failed: %lu\n",
                     GetLastError());
        return false;
      }
      if (!accounting.ActiveProcesses)
        break;
      if (GetTickCount64() >= deadline) {
        std::fputs("test-user: descendants did not exit\n", stderr);
        return false;
      }
      Sleep(10);
    }
    handle = Handle();
    return true;
  }
  ~ProcessTree() { close(); }
};
int wmain(int argc, wchar_t **argv) {
  try {
    const bool standard =
        argc > 1 && std::wstring(argv[1]) == L"--standard-user";
    const int logIndex = standard ? 3 : 1, executableIndex = logIndex + 1;
    require(argc > executableIndex, "Usage: test-user [--standard-user "
                                    "checkout-root] log executable args...");
    HANDLE original = nullptr, restricted = nullptr;
    require(OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &original),
            "Open token");
    Handle token(original);
    std::unique_ptr<test::Account> account;
    Handle childToken;
    if (standard) {
      account = std::make_unique<test::Account>(token.value);
      require(DuplicateHandle(GetCurrentProcess(), account->token.value,
                              GetCurrentProcess(), &restricted, 0, FALSE,
                              DUPLICATE_SAME_ACCESS),
              "Copy standard token");
      childToken = Handle(restricted);
    } else {
      require(CreateRestrictedToken(
                  token.value, LUA_TOKEN | DISABLE_MAX_PRIVILEGE, 0, nullptr, 0,
                  nullptr, 0, nullptr, &restricted),
              "LUA token");
      childToken = Handle(restricted);
    }
    if (!standard) {
      PSID sid = nullptr;
      require(ConvertStringSidToSidW(L"S-1-16-8192", &sid),
              "Medium integrity SID");
      TOKEN_MANDATORY_LABEL label{};
      label.Label.Sid = sid;
      label.Label.Attributes = SE_GROUP_INTEGRITY;
      const bool lowered =
          SetTokenInformation(childToken.value, TokenIntegrityLevel, &label,
                              sizeof(label) + GetLengthSid(sid));
      LocalFree(sid);
      require(lowered, "Set medium integrity");
    }
    const auto user = tokenInformation(childToken.value, TokenUser);
    const auto groups = tokenInformation(childToken.value, TokenGroups);
    auto *groupList = reinterpret_cast<const TOKEN_GROUPS *>(groups.data());
    PSID logon = nullptr;
    for (DWORD i = 0; i < groupList->GroupCount; ++i)
      if ((groupList->Groups[i].Attributes & SE_GROUP_LOGON_ID) ==
          SE_GROUP_LOGON_ID)
        logon = groupList->Groups[i].Sid;
    require(logon != nullptr, "Child logon SID");
    if (!standard) {
      LPWSTR userText = nullptr;
      require(ConvertSidToStringSidW(
                  reinterpret_cast<const TOKEN_USER *>(user.data())->User.Sid,
                  &userText),
              "Child user SID");
      const auto daclText =
          L"D:P(A;;GA;;;SY)(A;;GA;;;" + std::wstring(userText) + L")";
      LocalFree(userText);
      PSECURITY_DESCRIPTOR defaults = nullptr;
      require(ConvertStringSecurityDescriptorToSecurityDescriptorW(
                  daclText.c_str(), SDDL_REVISION_1, &defaults, nullptr),
              "Child default security");
      TOKEN_DEFAULT_DACL defaultDacl{};
      BOOL present = FALSE, defaulted = FALSE;
      const bool changed =
          GetSecurityDescriptorDacl(defaults, &present,
                                    &defaultDacl.DefaultDacl, &defaulted) &&
          present &&
          SetTokenInformation(childToken.value, TokenDefaultDacl, &defaultDacl,
                              sizeof(defaultDacl));
      LocalFree(defaults);
      require(changed, "Set child default DACL");
    }
    auto *desktopSid =
        standard ? reinterpret_cast<const TOKEN_USER *>(user.data())->User.Sid
                 : logon;
    DesktopAccess station(GetProcessWindowStation(), desktopSid,
                          WINSTA_ALL_ACCESS);
    DesktopAccess desktop(GetThreadDesktop(GetCurrentThreadId()), desktopSid,
                          GENERIC_ALL);
    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    Handle log(CreateFileW(argv[logIndex], GENERIC_WRITE,
                           FILE_SHARE_READ | FILE_SHARE_WRITE, &attributes,
                           CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr));
    Handle input(CreateFileW(L"NUL", GENERIC_READ,
                             FILE_SHARE_READ | FILE_SHARE_WRITE, &attributes,
                             OPEN_EXISTING, 0, nullptr));
    require(bool(log) && bool(input), "Child stdio");
    SIZE_T size = 0;
    InitializeProcThreadAttributeList(nullptr, 1, 0, &size);
    std::vector<uint8_t> storage(size);
    auto *list = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
    require(InitializeProcThreadAttributeList(list, 1, 0, &size),
            "Initialize handle list");
    HANDLE handles[] = {input.value, log.value};
    require(
        UpdateProcThreadAttribute(list, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                                  handles, sizeof(handles), nullptr, nullptr),
        "Child handle list");
    std::wstring command;
    for (int i = executableIndex; i < argc; ++i) {
      if (!command.empty())
        command += L' ';
      command += quote(argv[i]);
    }
    STARTUPINFOEXW startup{};
    startup.StartupInfo.cb = sizeof(startup);
    startup.lpAttributeList = list;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = input.value;
    startup.StartupInfo.hStdOutput = startup.StartupInfo.hStdError = log.value;
    std::unique_ptr<test::FileAccess> sourceAccess, logAccess;
    std::vector<wchar_t> environment;
    if (standard) {
      const auto *userData = reinterpret_cast<const TOKEN_USER *>(user.data());
      sourceAccess = std::make_unique<test::FileAccess>(
          canonicalPath(argv[2]), userData->User.Sid,
          FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
          SUB_CONTAINERS_AND_OBJECTS_INHERIT);
      logAccess = std::make_unique<test::FileAccess>(
          canonicalPath(argv[logIndex]), userData->User.Sid,
          FILE_GENERIC_READ | FILE_GENERIC_WRITE, NO_INHERITANCE);
      environment = account->childEnvironment(canonicalPath(argv[logIndex]));
    }
    ProcessTree containment;
    PROCESS_INFORMATION process{};
    bool created = false;
    if (standard) {
      STARTUPINFOW normal{};
      normal.cb = sizeof(normal);
      created = CreateProcessWithLogonW(
          account->name.c_str(), L".", account->password.c_str(),
          LOGON_WITH_PROFILE, argv[executableIndex], command.data(),
          CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED, environment.data(),
          nullptr, &normal, &process);
      SecureZeroMemory(account->password.data(),
                       account->password.size() * sizeof(wchar_t));
    } else {
      created = CreateProcessAsUserW(
          childToken.value, argv[executableIndex], command.data(), nullptr,
          nullptr, TRUE, EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
          nullptr, nullptr, &startup.StartupInfo, &process);
    }
    const DWORD creationError = GetLastError();
    DeleteProcThreadAttributeList(list);
    if (!created) {
      SetLastError(creationError);
      require(false, "Start medium-integrity test");
    }
    Handle running(process.hProcess), thread(process.hThread);
    if (!AssignProcessToJobObject(containment.handle.value, running.value)) {
      const auto assignmentError = GetLastError();
      require(TerminateProcess(running.value, 1) &&
                  WaitForSingleObject(running.value, 10000) == WAIT_OBJECT_0,
              "Reap uncontained test root");
      SetLastError(assignmentError);
      require(false, "Assign test process tree");
    }
    if (standard) {
      Identity::process(running.value);
      HANDLE actualRaw = nullptr;
      require(OpenProcessToken(running.value, TOKEN_QUERY, &actualRaw),
              "Query actual standard token");
      Handle actual(actualRaw);
      const auto actualUser = tokenInformation(actual.value, TokenUser);
      require(
          EqualSid(
              reinterpret_cast<const TOKEN_USER *>(actualUser.data())->User.Sid,
              reinterpret_cast<const TOKEN_USER *>(user.data())->User.Sid),
          "Unexpected actual user");
      require(!IsTokenRestricted(actual.value),
              "Standard test token must be unrestricted");
    }

    require(ResumeThread(thread.value) != static_cast<DWORD>(-1),
            "Resume test root");

    const DWORD waited = WaitForSingleObject(running.value, 180000);
    if (waited != WAIT_OBJECT_0) {
      TerminateProcess(running.value, 124);
      throw std::runtime_error("Medium test timed out");
    }
    DWORD exitCode = 1;
    require(GetExitCodeProcess(running.value, &exitCode), "Child exit code");
    bool cleaned = containment.close();
    running = Handle();
    thread = Handle();
    if (logAccess)
      cleaned &= logAccess->close();
    if (sourceAccess)
      cleaned &= sourceAccess->close();
    cleaned &= desktop.close();
    cleaned &= station.close();
    if (account)
      cleaned &= account->close();
    std::ifstream result{std::filesystem::path(argv[logIndex])};
    if (standard && exitCode == 0)
      require(result.peek() != std::ifstream::traits_type::eof(),
              "Missing standard-user test output");
    std::string line;
    while (std::getline(result, line))
      std::printf("%s\n", line.c_str());
    require(cleaned, "Standard-user fixture cleanup");
    return static_cast<int>(exitCode);
  } catch (const std::exception &error) {
    std::fprintf(stderr, "test-user: %s\n", error.what());
    return 1;
  }
}
