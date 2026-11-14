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

void Config::normalize() {
  if (server == "Custom") {
    const auto first = host.find_first_not_of(" \t\r\n");
    host =
        first == std::string::npos
            ? ""
            : host.substr(first, host.find_last_not_of(" \t\r\n") - first + 1);
  }
}
void Config::validate() const {
  if (inlineCandidates < 0)
    throw std::runtime_error("候補数は0以上の整数で指定してください。");
  if (port < 1 || port > 65535)
    throw std::runtime_error("ポートは1〜65535で指定してください。");
  if (timeout < 0 || timeout > 1000)
    throw std::runtime_error("候補を待つ時間は0〜1000 msで指定してください。");
  if (server != "Google" && server != "Custom" && server != "Disabled")
    throw std::runtime_error(
        "SKKサーバーはGoogle、Custom、Disabledから選択してください。");
  if (encoding != "euc-jp" && encoding != "utf-8")
    throw std::runtime_error(
        "辞書の文字コードはeuc-jpかutf-8を選択してください。");
  if (serverEncoding != "euc-jp" && serverEncoding != "utf-8")
    throw std::runtime_error(
        "サーバーの文字コードはeuc-jpかutf-8を選択してください。");
}
void Config::validateForSave() const {
  validate();
  const auto path = [](const std::string &value, const char *label) {
    if (value.empty() || value.find_first_of("\r\n") != std::string::npos ||
        value.find('\0') != std::string::npos)
      throw std::runtime_error(std::string(label) +
                               ": 空欄、改行、NULを含むパスは指定できません。");
  };
  path(dictionary, "辞書");
  if (!userDictionaryAutomatic)
    path(userDictionary, "ユーザー辞書");
  if (!modelAutomatic)
    path(model, "モデル");
  if ((server == "Custom" && host.empty()) ||
      host.find_first_of("\r\n") != std::string::npos ||
      host.find('\0') != std::string::npos)
    throw std::runtime_error(
        "カスタムサーバーのホストを指定してください。改行やNULは使えません。");
}

namespace {
Config readConfig(std::string *warning) {
  Config c;
  c.userDictionary =
      std::string(g_get_user_data_dir()) + "/ibus/tgskk/SKK-JISYO.user";
  c.model = TGSKK_SYSTEM_MODEL;
  auto owned = readFile();
  auto *file = owned.get();
  GError *error = nullptr;
  auto fieldError = [&](const char *key) {
    if (!error)
      return;
    if (!warning)
      check(error);
    *warning += std::string(key) + ": " + error->message + "\n";
    g_clear_error(&error);
  };
  auto string = [&](const char *key, std::string &value) {
    if (!error && g_key_file_has_key(file, "General", key, nullptr)) {
      gchar *text = g_key_file_get_string(file, "General", key, &error);
      if (text) {
        value = text;
        g_free(text);
      }
      if (error && std::string(key) == "SkkservMode")
        value = "Disabled";
      fieldError(key);
    }
  };
  auto integer = [&](const char *key, int &value) {
    if (g_key_file_has_key(file, "General", key, nullptr)) {
      const auto parsed = g_key_file_get_integer(file, "General", key, &error);
      if (!error)
        value = parsed;
      fieldError(key);
    }
  };
  auto boolean = [&](const char *key, bool &value) {
    if (g_key_file_has_key(file, "General", key, nullptr)) {
      value = g_key_file_get_boolean(file, "General", key, &error);
      fieldError(key);
    }
  };
  string("Dictionary", c.dictionary);
  // GLib reserves "Encoding" in the first group for the file encoding.
  string("DictionaryEncoding", c.encoding);
  std::string userOverride, modelOverride;
  string("UserDictionary", userOverride);
  string("ZenzModel", modelOverride);
  c.userDictionaryAutomatic = userOverride.empty();
  c.modelAutomatic = modelOverride.empty();
  if (!c.userDictionaryAutomatic)
    c.userDictionary = userOverride;
  if (!c.modelAutomatic)
    c.model = modelOverride;
  string("SkkservMode", c.server);
  string("SkkservHost", c.host);
  string("SkkservEncoding", c.serverEncoding);
  integer("InlineCandidates", c.inlineCandidates);
  integer("SkkservPort", c.port);
  integer("SkkservTimeoutMs", c.timeout);
  boolean("ZenzEnabled", c.ai);
  boolean("ModeIndicator", c.indicator);
  check(error);
  if (c.modelAutomatic) {
    const auto model = std::filesystem::path(g_get_user_data_dir()) /
                       "fcitx5/tgskk/zenz-v3.2-small-Q5_K_M.gguf";
    std::error_code status;
    if (std::filesystem::is_regular_file(model, status))
      c.model = model.string();
  }
  c.normalize();
  return c;
}
} // namespace
Config Config::load() {
  auto config = readConfig(nullptr);
  config.validate();
  return config;
}
Config Config::loadForSettings(std::string &warning) {
  warning.clear();
  auto config = readConfig(&warning);
  try {
    config.validateForSave();
  } catch (const std::exception &error) {
    warning += error.what();
  }
  return config;
}

void Config::save(const Config &value) {
  auto c = value;
  c.normalize();
  c.validateForSave();
  auto file = readFile();
  const auto string = [&](const char *key, const std::string &value) {
    g_key_file_set_string(file.get(), "General", key, value.c_str());
  };
  string("Dictionary", c.dictionary);
  string("DictionaryEncoding", c.encoding);
  if (c.userDictionaryAutomatic)
    g_key_file_remove_key(file.get(), "General", "UserDictionary", nullptr);
  else
    string("UserDictionary", c.userDictionary);
  if (c.modelAutomatic)
    g_key_file_remove_key(file.get(), "General", "ZenzModel", nullptr);
  else
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
