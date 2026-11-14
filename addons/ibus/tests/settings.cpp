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
  const auto original = bytes(path);
  config.port = 70000;
  rejected(config);
  config.port = 1179;
  config.inlineCandidates = 65;
  rejected(config);
  config.inlineCandidates = 0;
  config.host = " \t";
  rejected(config);
  config.host = "localhost";
  config.dictionary = "relative";
  rejected(config);
  config.dictionary = restored.dictionary;
  g_assert_true(bytes(path) == original);
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
  g_assert_true(bytes(config.userDictionary) == ";; learned sentinel\n");
  gtk_widget_destroy(window);
  std::ofstream(path) << "[malformed";
  const auto malformed = bytes(path);
  rejected(config);
  g_assert_true(bytes(path) == malformed);
  std::filesystem::remove_all(root);
}
