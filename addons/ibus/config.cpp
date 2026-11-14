// SPDX-License-Identifier: GPL-3.0-or-later
#include "config.h"
#include "paths.h"
#include <filesystem>
#include <gio/gio.h>
#include <memory>
#include <stdexcept>

namespace tgskk_ibus {
namespace {
std::string configPath() {
  return std::string(g_get_user_config_dir()) + "/tgskk/ibus.conf";
}
void check(GError *error) {
  if (error) {
    const std::string message = error->message;
    g_error_free(error);
    throw std::runtime_error(message);
  }
}
using KeyFile = std::unique_ptr<GKeyFile, decltype(&g_key_file_unref)>;
KeyFile readFile() {
  KeyFile file(g_key_file_new(), g_key_file_unref);
  GError *error = nullptr;
  if (!g_key_file_load_from_file(file.get(), configPath().c_str(),
                                 GKeyFileFlags(G_KEY_FILE_KEEP_COMMENTS |
                                               G_KEY_FILE_KEEP_TRANSLATIONS),
                                 &error)) {
    if (g_error_matches(error, G_FILE_ERROR, G_FILE_ERROR_NOENT))
      g_clear_error(&error);
    check(error);
  }
  return file;
}
} // namespace

void Config::validate() const {
  const auto path = [](const std::string &value) {
    return !value.empty() && value.front() == '/' &&
           value.find_first_of("\r\n") == std::string::npos &&
           value.find('\0') == std::string::npos;
  };
  if (!path(dictionary) || !path(userDictionary) || !path(model) ||
      inlineCandidates < 0 || inlineCandidates > 64 || port < 1 ||
      port > 65535 || timeout < 0 || timeout > 1000 ||
      (server != "Google" && server != "Custom" && server != "Disabled") ||
      (server == "Custom" &&
       host.find_first_not_of(" \t\r\n") == std::string::npos) ||
      host.find_first_of("\r\n") != std::string::npos ||
      host.find('\0') != std::string::npos ||
      (encoding != "euc-jp" && encoding != "utf-8") ||
      (serverEncoding != "euc-jp" && serverEncoding != "utf-8"))
    throw std::runtime_error("設定値が不正です。絶対パス、文字コード、サーバー"
                             "と数値の範囲を確認してください。");
}

Config Config::load() {
  Config c;
  c.userDictionary =
      std::string(g_get_user_data_dir()) + "/ibus/tgskk/SKK-JISYO.user";
  c.model = TGSKK_SYSTEM_MODEL;
  auto owned = readFile();
  auto *file = owned.get();
  GError *error = nullptr;
  auto string = [&](const char *key, std::string &value) {
    if (!error && g_key_file_has_key(file, "General", key, nullptr)) {
      gchar *text = g_key_file_get_string(file, "General", key, &error);
      if (text) {
        value = text;
        g_free(text);
      }
    }
  };
  auto integer = [&](const char *key, int &value) {
    if (!error && g_key_file_has_key(file, "General", key, nullptr))
      value = g_key_file_get_integer(file, "General", key, &error);
  };
  auto boolean = [&](const char *key, bool &value) {
    if (!error && g_key_file_has_key(file, "General", key, nullptr))
      value = g_key_file_get_boolean(file, "General", key, &error);
  };
  string("Dictionary", c.dictionary);
  // GLib reserves "Encoding" in the first group for the file encoding.
  string("DictionaryEncoding", c.encoding);
  string("UserDictionary", c.userDictionary);
  string("ZenzModel", c.model);
  string("SkkservMode", c.server);
  string("SkkservHost", c.host);
  string("SkkservEncoding", c.serverEncoding);
  integer("InlineCandidates", c.inlineCandidates);
  integer("SkkservPort", c.port);
  integer("SkkservTimeoutMs", c.timeout);
  boolean("ZenzEnabled", c.ai);
  boolean("ModeIndicator", c.indicator);
  check(error);
  if (!g_key_file_has_key(file, "General", "ZenzModel", nullptr)) {
    const auto model = std::filesystem::path(g_get_user_data_dir()) /
                       "fcitx5/tgskk/zenz-v3.2-small-Q5_K_M.gguf";
    std::error_code status;
    if (std::filesystem::is_regular_file(model, status))
      c.model = model.string();
  }
  c.validate();
  return c;
}

void Config::save(const Config &c) {
  c.validate();
  auto file = readFile();
  const auto string = [&](const char *key, const std::string &value) {
    g_key_file_set_string(file.get(), "General", key, value.c_str());
  };
  string("Dictionary", c.dictionary);
  string("DictionaryEncoding", c.encoding);
  string("UserDictionary", c.userDictionary);
  string("ZenzModel", c.model);
  string("SkkservMode", c.server);
  string("SkkservHost", c.host);
  string("SkkservEncoding", c.serverEncoding);
  g_key_file_set_integer(file.get(), "General", "InlineCandidates",
                         c.inlineCandidates);
  g_key_file_set_integer(file.get(), "General", "SkkservPort", c.port);
  g_key_file_set_integer(file.get(), "General", "SkkservTimeoutMs", c.timeout);
  g_key_file_set_boolean(file.get(), "General", "ZenzEnabled", c.ai);
  g_key_file_set_boolean(file.get(), "General", "ModeIndicator", c.indicator);
  gsize length = 0;
  GError *error = nullptr;
  std::unique_ptr<gchar, decltype(&g_free)> data(
      g_key_file_to_data(file.get(), &length, &error), g_free);
  check(error);
  std::filesystem::create_directories(
      std::filesystem::path(configPath()).parent_path());
  g_file_set_contents_full(
      configPath().c_str(), data.get(), length,
      GFileSetContentsFlags(G_FILE_SET_CONTENTS_CONSISTENT |
                            G_FILE_SET_CONTENTS_DURABLE),
      0600, &error);
  check(error);
}
} // namespace tgskk_ibus
