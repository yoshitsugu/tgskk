#pragma once
#include "tgskk.h"
#include "../common/candidates.h"
#include <ibus.h>
#include <memory>
#include <string>
namespace tgskk_ibus {
struct Config {
  std::string dictionary = "/usr/share/skk/SKK-JISYO.L", encoding = "euc-jp";
  std::string userDictionary, model, server = "Google", host = "127.0.0.1",
                                     serverEncoding = "euc-jp";
  int inlineCandidates = static_cast<int>(tgskk::defaultInlineCandidates), port = 1178, timeout = 300;
  bool ai = true, indicator = true;
  static Config load();
};
struct Resources {
  Config config;
  TgskkRuntime *runtime = nullptr;
  TgskkZenz *ai = nullptr;
  TgskkSkkserv *server = nullptr;
  guint saveTimer = 0;
  explicit Resources(Config config);
  ~Resources();
  void save();
  void scheduleSave();
};
extern std::unique_ptr<Resources> resources;
GType engineType();
} // namespace tgskk_ibus
