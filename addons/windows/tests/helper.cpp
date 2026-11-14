// SPDX-License-Identifier: GPL-3.0-or-later
#include <cstdio>
#include <cwchar>
#include <fcntl.h>
#include <io.h>
#include <string>
#include <vector>
#include <windows.h>
int wmain(int argc, wchar_t **argv) {
  if (argc < 2)
    return 2;
  if (_setmode(_fileno(stdin), _O_BINARY) == -1 ||
      _setmode(_fileno(stdout), _O_BINARY) == -1)
    return 1;
  if (!std::wcscmp(argv[1], L"stall")) {
    Sleep(30000);
    return 0;
  }
  if (!std::wcscmp(argv[1], L"write-stall")) {
    std::fwrite("READY", 1, 5, stdout);
    std::fflush(stdout);
    Sleep(30000);
    return 0;
  }
  if (!std::wcscmp(argv[1], L"partial")) {
    std::fwrite("abc", 1, 3, stdout);
    std::fflush(stdout);
    return 0;
  }
  if (!std::wcscmp(argv[1], L"echo")) {
    char buffer[5];
    if (std::fread(buffer, 1, 5, stdin) != 5)
      return 1;
    std::fwrite(buffer, 1, 5, stdout);
    return std::fflush(stdout) ? 1 : 0;
  }
  if (!std::wcscmp(argv[1], L"--stdio")) {
    if (argc != 3 || std::wcscmp(argv[2], L"--encoding=utf-8"))
      return 2;
    unsigned requests = 0;
    for (;;) {
      int code = std::fgetc(stdin);
      if (code == EOF || code == '0')
        return 0;
      if (code != '1')
        return 1;
      unsigned count = 0;
      std::string reading;
      for (;;) {
        int ch = std::fgetc(stdin);
        if (ch == EOF || ++count > 4096)
          return 1;
        if (ch == ' ')
          break;
        reading.push_back(static_cast<char>(ch));
      }
      if (requests == 0 && reading == "\xe3\x81\x8d\xe3\x81\x97\xe3\x82\x83")
        std::fputs("1/\xe6\xb1\xbd\xe8\xbb\x8a/\n", stdout);
      else if (requests == 1 &&
               reading == "\xe3\x81\x8b\xe3\x82\x93\xe3\x81\x98")
        std::fputs("1/\xe6\xbc\xa2\xe5\xad\x97;\xe6\x96\x87\xe5\xad\x97/\n",
                   stdout);
      else
        return 3;
      ++requests;
      if (std::fflush(stdout))
        return 1;
    }
  }
  return 2;
}
