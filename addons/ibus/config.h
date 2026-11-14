#pragma once
#include <string>
namespace tgskk_ibus {
struct Config {
  std::string dictionary = "/usr/share/skk/SKK-JISYO.L", encoding = "euc-jp";
  std::string userDictionary, model, server = "Google", host = "127.0.0.1",
                                     serverEncoding = "euc-jp";
  int inlineCandidates = 4, port = 1178, timeout = 300;
  bool ai = true, indicator = true;
  static Config load();
  static void save(const Config &config);
  void validate() const;
};
} // namespace tgskk_ibus
