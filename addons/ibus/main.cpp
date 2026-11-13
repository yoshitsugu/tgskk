// SPDX-License-Identifier: GPL-3.0-or-later
#include "engine.h"
#include <csignal>
#include <cstdio>
#include <glib-unix.h>
int main(int argc, char **argv) {
  if (argc != 2 || std::string(argv[1]) != "--ibus") {
    std::fprintf(stderr, "Usage: ibus-engine-tgskk --ibus\n");
    return 2;
  }
  ibus_init();
  try {
    tgskk_ibus::resources =
        std::make_unique<tgskk_ibus::Resources>(tgskk_ibus::Config::load());
  } catch (const std::exception &error) {
    std::fprintf(stderr, "tgskk: %s\n", error.what());
    return 1;
  }
  auto *bus = ibus_bus_new();
  if (!ibus_bus_is_connected(bus)) {
    std::fprintf(stderr, "tgskk: IBus is not connected\n");
    g_object_unref(bus);
    return 1;
  }
  g_signal_connect(bus, "disconnected",
                   G_CALLBACK(+[](IBusBus *, gpointer) { ibus_quit(); }),
                   nullptr);
  auto *factory = ibus_factory_new(ibus_bus_get_connection(bus));
  ibus_factory_add_engine(factory, "tgskk", tgskk_ibus::engineType());
  if (ibus_bus_request_name(bus, "org.freedesktop.IBus.Tgskk",
                            IBUS_BUS_NAME_FLAG_DO_NOT_QUEUE) !=
      IBUS_BUS_REQUEST_NAME_REPLY_PRIMARY_OWNER) {
    g_object_unref(factory);
    g_object_unref(bus);
    return 1;
  }
  guint term = g_unix_signal_add(
      SIGTERM,
      +[](gpointer) -> gboolean {
        ibus_quit();
        return G_SOURCE_CONTINUE;
      },
      nullptr);
  guint interrupt = g_unix_signal_add(
      SIGINT,
      +[](gpointer) -> gboolean {
        ibus_quit();
        return G_SOURCE_CONTINUE;
      },
      nullptr);
  ibus_main();
  g_source_remove(term);
  g_source_remove(interrupt);
  ibus_object_destroy(IBUS_OBJECT(factory));
  g_object_unref(factory);
  g_object_unref(bus);
  tgskk_ibus::resources.reset();
}
