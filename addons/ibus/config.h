#pragma once
#include "../common/candidates.h"
#include <string>
namespace tgskk_ibus {
struct Config {
  std::string dictionary = "/usr/share/skk/SKK-JISYO.L", encoding = "euc-jp";
  std::string userDictionary, model, server = "Google", host = "127.0.0.1",
                                     serverEncoding = "euc-jp";
  int inlineCandidates = static_cast<int>(tgskk::defaultInlineCandidates),
      port = 1178, timeout = 300;
  bool ai = true, indicator = true;
  bool userDictionaryAutomatic = true, modelAutomatic = true;
  static Config load();
  static Config loadForSettings(std::string &warning);
  static void save(const Config &config);
  void validate() const;
  void validateForSave() const;
  void normalize();
};
} // namespace tgskk_ibus
