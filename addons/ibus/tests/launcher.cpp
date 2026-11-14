#include <gio/gdesktopappinfo.h>
#include <ibus.h>
#include <string>
int main(int argc, char **argv) {
  g_assert_cmpint(argc, ==, 5);
  ibus_init();
  auto *component = ibus_component_new_from_file(argv[1]);
  g_assert_nonnull(component);
  auto *engines = ibus_component_get_engines(component);
  g_assert_nonnull(engines);
  auto *engine = IBUS_ENGINE_DESC(engines->data);
  g_assert_cmpstr(ibus_engine_desc_get_name(engine), ==, "tgskk");
  g_assert_cmpstr(ibus_engine_desc_get_setup(engine), ==, argv[3]);
  auto *file = g_key_file_new();
  g_assert_true(
      g_key_file_load_from_file(file, argv[2], G_KEY_FILE_NONE, nullptr));
  auto *command = g_key_file_get_string(file, "Desktop Entry", "Exec", nullptr);
  gchar **arguments = nullptr;
  int count = 0;
  g_assert_true(g_shell_parse_argv(command, &count, &arguments, nullptr));
  g_assert_cmpint(count, ==, 1);
  g_assert_cmpstr(arguments[0], ==, argv[3]);
  g_strfreev(arguments);
  g_free(command);
  const auto *executable = g_getenv("TGSKK_TEST_INSTALLED") ? argv[3] : argv[4];
  // GDesktopAppInfo rejects uninstalled absolute executables even with valid
  // metadata.
  const auto quoted = std::string("\"") + executable + "\"";
  g_key_file_set_string(file, "Desktop Entry", "Exec", quoted.c_str());
  auto *desktop = g_desktop_app_info_new_from_keyfile(file);
  g_assert_nonnull(desktop);
  g_assert_cmpstr(g_app_info_get_commandline(G_APP_INFO(desktop)), ==,
                  quoted.c_str());
  g_key_file_unref(file);
  g_assert_true(g_desktop_app_info_get_nodisplay(desktop));
  g_object_unref(desktop);
  g_list_free(engines);
  g_object_unref(component);
}
