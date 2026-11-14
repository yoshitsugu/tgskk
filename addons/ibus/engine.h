#pragma once
#include "tgskk.h"
#include "config.h"
#include <ibus.h>
#include <memory>
#include <string>
namespace tgskk_ibus {
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
