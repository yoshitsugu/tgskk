#include "settings.h"
#include "config.h"
#include <csignal>
#include <filesystem>
#include <fstream>
#include <string>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

namespace {
std::string bytes(const std::string &path) {
  std::ifstream file(path);
  return {std::istreambuf_iterator<char>(file), {}};
}
GtkWidget *find(GtkWidget *widget, const char *name) {
  if (g_strcmp0(gtk_widget_get_name(widget), name) == 0)
    return widget;
  if (!GTK_IS_CONTAINER(widget))
    return nullptr;
  auto *children = gtk_container_get_children(GTK_CONTAINER(widget));
  GtkWidget *found = nullptr;
  for (auto *child = children; child && !found; child = child->next)
    found = find(GTK_WIDGET(child->data), name);
  g_list_free(children);
  return found;
}
void rejected(tgskk_ibus::Config config) {
  bool failed = false;
  try {
    tgskk_ibus::Config::save(config);
  } catch (const std::exception &) {
    failed = true;
  }
  g_assert_true(failed);
}
} // namespace
int main(int argc, char **argv) {
  auto *temporary = g_dir_make_tmp("tgskk-settings-XXXXXX", nullptr);
  g_assert_nonnull(temporary);
  const std::string root = temporary;
  g_free(temporary);
  g_setenv("XDG_CONFIG_HOME", (root + "/config").c_str(), TRUE);
  g_setenv("XDG_DATA_HOME", (root + "/data").c_str(), TRUE);
  // These tests must not spawn the host's IBus or use its input method.
  g_setenv("GDK_BACKEND", "x11", TRUE);
  g_setenv("GTK_IM_MODULE", "gtk-im-context-simple", TRUE);
  const auto path = root + "/config/tgskk/ibus.conf";
  auto config = tgskk_ibus::Config::load();
  std::filesystem::create_directories(root + "/config/tgskk");
  std::ofstream(path) << "# retain comment\n[General]\nFutureOption=retain\n";
  std::filesystem::create_directories(
      std::filesystem::path(config.userDictionary).parent_path());
  std::ofstream(config.userDictionary) << ";; learned sentinel\n";
  config.ai = false;
  config.server = "Custom";
  config.host = "localhost";
  config.port = 1179;
  config.encoding = "utf-8";
  config.inlineCandidates = 0;
  tgskk_ibus::Config::save(config);
  const auto restored = tgskk_ibus::Config::load();
  g_assert_false(restored.ai);
  g_assert_cmpstr(restored.server.c_str(), ==, "Custom");
  g_assert_cmpint(restored.port, ==, 1179);
  g_assert_cmpint(restored.inlineCandidates, ==, 0);
  g_assert_cmpstr(restored.encoding.c_str(), ==, "utf-8");
  g_assert_true(bytes(path).find("# retain comment") != std::string::npos);
  g_assert_true(bytes(path).find("FutureOption=retain") != std::string::npos);
  g_assert_true(restored.modelAutomatic);
  g_assert_true(restored.userDictionaryAutomatic);
  g_assert_true(bytes(path).find("ZenzModel=") == std::string::npos);
  g_assert_true(bytes(path).find("UserDictionary=") == std::string::npos);
  const auto localModel =
      root + "/data/fcitx5/tgskk/zenz-v3.2-small-Q5_K_M.gguf";
  std::filesystem::create_directories(
      std::filesystem::path(localModel).parent_path());
  std::ofstream(localModel) << "newly installed model";
  g_assert_true(tgskk_ibus::Config::load().model == localModel);
  config.modelAutomatic = false;
  config.model = root + "/manual.gguf";
  config.userDictionaryAutomatic = false;
  const auto learned = config.userDictionary;
  config.userDictionary = root + "/manual.user";
  config.host = " localhost ";
  tgskk_ibus::Config::save(config);
  auto manual = tgskk_ibus::Config::load();
  g_assert_false(manual.modelAutomatic);
  g_assert_false(manual.userDictionaryAutomatic);
  g_assert_true(manual.model == config.model);
  g_assert_true(manual.userDictionary == config.userDictionary);
  g_assert_cmpstr(manual.host.c_str(), ==, "localhost");
  config.modelAutomatic = true;
  config.userDictionaryAutomatic = true;
  config.userDictionary = learned;
  config.host = "localhost";
  tgskk_ibus::Config::save(config);
  g_assert_true(tgskk_ibus::Config::load().model == localModel);
  const auto original = bytes(path);
  config.port = 70000;
  rejected(config);
  config.port = 1179;
  config.inlineCandidates = -1;
  rejected(config);
  config.inlineCandidates = 0;
  config.host = " \t";
  rejected(config);
  config.server = "Disabled";
  config.host = std::string("local\0suffix", 12);
  rejected(config);
  config.server = "Custom";
  config.host = "localhost";
  config.dictionary = "";
  rejected(config);
  config.dictionary = std::string("bad\0path", 8);
  rejected(config);
  config.dictionary = restored.dictionary;
  g_assert_true(bytes(path) == original);
  std::ofstream(path) << "[General]\nInlineCandidates=100\nDictionary=relative."
                         "dict\nSkkservMode=Custom\nSkkservHost=\n";
  const auto legacy = tgskk_ibus::Config::load();
  g_assert_cmpint(legacy.inlineCandidates, ==, 100);
  g_assert_cmpstr(legacy.dictionary.c_str(), ==, "relative.dict");
  g_assert_true(legacy.host.empty());
  std::ofstream(path) << original;
  const auto child = fork();
  g_assert_cmpint(child, >=, 0);
  if (child == 0) {
    signal(SIGXFSZ, SIG_IGN);
    rlimit limit{0, 0};
    if (setrlimit(RLIMIT_FSIZE, &limit) != 0)
      _exit(2);
    config.indicator = false;
    try {
      tgskk_ibus::Config::save(config);
    } catch (const std::exception &) {
      _exit(0);
    }
    _exit(1);
  }
  int status = 0;
  g_assert_cmpint(waitpid(child, &status, 0), ==, child);
  g_assert_true(WIFEXITED(status));
  g_assert_cmpint(WEXITSTATUS(status), ==, 0);
  g_assert_true(bytes(path) == original);
  gtk_init(&argc, &argv);
  auto *window = tgskk_ibus::createSettingsWindow();
  gtk_widget_show_all(window);
  auto *inlineCandidates = find(window, "InlineCandidates");
  auto *save = find(window, "save");
  g_assert_nonnull(inlineCandidates);
  g_assert_nonnull(save);
  gtk_entry_set_text(GTK_ENTRY(inlineCandidates), "7");
  gtk_button_clicked(GTK_BUTTON(save));
  g_assert_cmpint(tgskk_ibus::Config::load().inlineCandidates, ==, 7);
  const auto saved = bytes(path);
  auto *port = find(window, "SkkservPort");
  const auto loseFocus = [&] {
    auto *event = gdk_event_new(GDK_FOCUS_CHANGE);
    event->focus_change.in = FALSE;
    event->focus_change.window =
        GDK_WINDOW(g_object_ref(gtk_widget_get_window(port)));
    gtk_widget_send_focus_change(port, event);
    gdk_event_free(event);
  };
  gtk_entry_set_text(GTK_ENTRY(port), "70000");
  loseFocus();
  gtk_button_clicked(GTK_BUTTON(save));
  g_assert_true(bytes(path) == saved);
  gtk_entry_set_text(GTK_ENTRY(port), "12x");
  loseFocus();
  gtk_button_clicked(GTK_BUTTON(save));
  g_assert_true(bytes(path) == saved);
  g_assert_cmpstr(gtk_label_get_text(GTK_LABEL(find(window, "status"))), !=,
                  "");
  gtk_entry_set_text(GTK_ENTRY(port), "99999999999");
  gtk_button_clicked(GTK_BUTTON(save));
  g_assert_true(bytes(path) == saved);
  g_assert_nonnull(g_strstr_len(
      gtk_label_get_text(GTK_LABEL(find(window, "status"))), -1, "ポート"));
  g_assert_true(bytes(config.userDictionary) == ";; learned sentinel\n");
  gtk_widget_destroy(window);
  std::ofstream(path) << "[General]\nSkkservPort=70000\nSkkservMode="
                         "Disabled\nZenzEnabled=false\n";
  const auto invalid = bytes(path);
  window = tgskk_ibus::createSettingsWindow();
  g_assert_true(bytes(path) == invalid);
  g_assert_cmpstr(gtk_entry_get_text(GTK_ENTRY(find(window, "SkkservPort"))),
                  ==, "70000");
  g_assert_cmpstr(gtk_label_get_text(GTK_LABEL(find(window, "status"))), !=,
                  "");
  gtk_entry_set_text(GTK_ENTRY(find(window, "SkkservPort")), "1178");
  gtk_button_clicked(GTK_BUTTON(find(window, "save")));
  g_assert_cmpint(tgskk_ibus::Config::load().port, ==, 1178);
  g_assert_true(tgskk_ibus::Config::load().modelAutomatic);
  gtk_widget_destroy(window);
  std::ofstream(path) << "[General]\nSkkservPort=not-an-integer\nSkkservMode="
                         "Disabled\nZenzEnabled=invalid\n";
  std::string warning;
  auto repair = tgskk_ibus::Config::loadForSettings(warning);
  g_assert_false(warning.empty());
  g_assert_false(repair.ai);
  window = tgskk_ibus::createSettingsWindow();
  gtk_button_clicked(GTK_BUTTON(find(window, "save")));
  g_assert_false(tgskk_ibus::Config::load().ai);
  gtk_widget_destroy(window);
  std::ofstream(path) << "[General]\nDictionaryEncoding=bogus\nSkkservMode="
                         "Disabled\nZenzEnabled=false\n";
  const auto invalidEncoding = bytes(path);
  window = tgskk_ibus::createSettingsWindow();
  gtk_button_clicked(GTK_BUTTON(find(window, "save")));
  g_assert_true(bytes(path) == invalidEncoding);
  g_assert_nonnull(
      g_strstr_len(gtk_label_get_text(GTK_LABEL(find(window, "status"))), -1,
                   "辞書の文字コード"));
  gtk_combo_box_set_active_id(GTK_COMBO_BOX(find(window, "DictionaryEncoding")),
                              "utf-8");
  gtk_button_clicked(GTK_BUTTON(find(window, "save")));
  const auto repairedEncoding = tgskk_ibus::Config::load();
  g_assert_cmpstr(repairedEncoding.encoding.c_str(), ==, "utf-8");
  gtk_widget_destroy(window);
  std::ofstream(path) << "[malformed";
  const auto malformed = bytes(path);
  rejected(config);
  g_assert_true(bytes(path) == malformed);
  std::filesystem::remove_all(root);
}
