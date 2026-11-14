// SPDX-License-Identifier: GPL-3.0-or-later
#include "settings.h"
#include <exception>

int main(int argc, char **argv) {
  gtk_init(&argc, &argv);
  try {
    auto *window = tgskk_ibus::createSettingsWindow();
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), nullptr);
    gtk_widget_show_all(window);
    gtk_main();
    return 0;
  } catch (const std::exception &error) {
    auto *dialog = gtk_message_dialog_new(
        nullptr, GTK_DIALOG_MODAL, GTK_MESSAGE_ERROR, GTK_BUTTONS_CLOSE,
        "設定を開けませんでした: %s", error.what());
    gtk_dialog_run(GTK_DIALOG(dialog));
    gtk_widget_destroy(dialog);
    return 1;
  }
}
