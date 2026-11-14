// SPDX-License-Identifier: GPL-3.0-or-later
#include "settings.h"
#include "config.h"
#include <array>
#include <memory>
#include <stdexcept>
#include <string>

namespace tgskk_ibus {
namespace {
struct Settings {
  Config config;
  GtkWidget *grid, *status;
  GtkWidget *dictionary, *encoding, *userDictionary, *model, *ai, *indicator;
  GtkWidget *server, *host, *port, *serverEncoding, *timeout, *inlineCandidates;
  int row = 0;
  GtkWidget *labelled(const char *label, const char *name, GtkWidget *widget) {
    auto *text = gtk_label_new_with_mnemonic(label);
    gtk_label_set_xalign(GTK_LABEL(text), 0);
    gtk_label_set_mnemonic_widget(GTK_LABEL(text), widget);
    gtk_widget_set_name(widget, name);
    gtk_widget_set_hexpand(widget, TRUE);
    gtk_grid_attach(GTK_GRID(grid), text, 0, row, 1, 1);
    gtk_grid_attach(GTK_GRID(grid), widget, 1, row++, 1, 1);
    return widget;
  }
  GtkWidget *entry(const char *label, const char *name,
                   const std::string &value, bool browse = false) {
    auto *widget = labelled(label, name, gtk_entry_new());
    gtk_entry_set_text(GTK_ENTRY(widget), value.c_str());
    if (browse) {
      auto *button = gtk_button_new_with_label("参照…");
      gtk_grid_attach(GTK_GRID(grid), button, 2, row - 1, 1, 1);
      g_signal_connect(
          button, "clicked", G_CALLBACK(+[](GtkButton *button, gpointer entry) {
            auto *parent =
                GTK_WINDOW(gtk_widget_get_toplevel(GTK_WIDGET(button)));
            const bool user =
                std::string(gtk_widget_get_name(GTK_WIDGET(entry))) ==
                "UserDictionary";
            auto *dialog = gtk_file_chooser_dialog_new(
                "ファイルを選択", parent,
                user ? GTK_FILE_CHOOSER_ACTION_SAVE
                     : GTK_FILE_CHOOSER_ACTION_OPEN,
                "キャンセル", GTK_RESPONSE_CANCEL, "選択", GTK_RESPONSE_ACCEPT,
                nullptr);
            const auto *current = gtk_entry_get_text(GTK_ENTRY(entry));
            if (g_path_is_absolute(current))
              gtk_file_chooser_set_filename(GTK_FILE_CHOOSER(dialog), current);
            if (gtk_dialog_run(GTK_DIALOG(dialog)) == GTK_RESPONSE_ACCEPT) {
              auto *path =
                  gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(dialog));
              if (path) {
                gtk_entry_set_text(GTK_ENTRY(entry), path);
                g_free(path);
              }
            }
            gtk_widget_destroy(dialog);
          }),
          widget);
    }
    return widget;
  }
  GtkWidget *combo(const char *label, const char *name, const char *first,
                   const char *second, const std::string &value,
                   const char *third = nullptr) {
    auto *widget = labelled(label, name, gtk_combo_box_text_new());
    for (const auto *item : {first, second, third})
      if (item)
        gtk_combo_box_text_append(GTK_COMBO_BOX_TEXT(widget), item, item);
    gtk_combo_box_set_active_id(GTK_COMBO_BOX(widget), value.c_str());
    return widget;
  }
  GtkWidget *toggle(const char *label, const char *name, bool value) {
    auto *widget = gtk_check_button_new_with_mnemonic(label);
    gtk_widget_set_name(widget, name);
    gtk_toggle_button_set_active(GTK_TOGGLE_BUTTON(widget), value);
    gtk_grid_attach(GTK_GRID(grid), widget, 0, row++, 3, 1);
    return widget;
  }
  GtkWidget *number(const char *label, const char *name, int value, int min,
                    int max) {
    auto *widget = entry(label, name, std::to_string(value));
    gtk_entry_set_input_purpose(GTK_ENTRY(widget), GTK_INPUT_PURPOSE_DIGITS);
    const auto range = std::to_string(min) + "–" + std::to_string(max);
    gtk_widget_set_tooltip_text(widget, range.c_str());
    return widget;
  }
  static std::string text(GtkWidget *widget) {
    return gtk_entry_get_text(GTK_ENTRY(widget));
  }
  static std::string selection(GtkWidget *widget) {
    const char *value = gtk_combo_box_get_active_id(GTK_COMBO_BOX(widget));
    if (!value)
      throw std::runtime_error("選択肢を指定してください。");
    return value;
  }
  void save() {
    try {
      auto next = config;
      next.dictionary = text(dictionary);
      next.encoding = selection(encoding);
      next.userDictionary = text(userDictionary);
      next.model = text(model);
      next.ai = gtk_toggle_button_get_active(GTK_TOGGLE_BUTTON(ai));
      next.indicator =
          gtk_toggle_button_get_active(GTK_TOGGLE_BUTTON(indicator));
      next.server = selection(server);
      next.host = text(host);
      next.serverEncoding = selection(serverEncoding);
      for (auto [widget, destination] :
           std::array<std::pair<GtkWidget *, int *>, 3>{
               std::pair{port, &next.port},
               {timeout, &next.timeout},
               {inlineCandidates, &next.inlineCandidates}}) {
        const auto value = text(widget);
        if (value.empty() ||
            value.find_first_not_of("0123456789") != std::string::npos)
          throw std::runtime_error("数値は範囲内の整数で指定してください。");
        *destination = std::stoi(value);
      }
      Config::save(next);
      config = std::move(next);
      gtk_label_set_text(GTK_LABEL(status),
                         "保存しました。入力を確定し、IBusを再起動するかログイ"
                         "ンし直すと反映されます。");
    } catch (const std::exception &error) {
      gtk_label_set_text(GTK_LABEL(status), error.what());
    }
  }
};
} // namespace

GtkWidget *createSettingsWindow() {
  auto state = std::make_unique<Settings>();
  state->config = Config::load();
  auto *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  gtk_window_set_title(GTK_WINDOW(window), "tgskk の設定（IBus）");
  gtk_window_set_default_size(GTK_WINDOW(window), 620, 680);
  auto *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
  gtk_container_set_border_width(GTK_CONTAINER(box), 16);
  gtk_container_add(GTK_CONTAINER(window), box);
  auto *scroll = gtk_scrolled_window_new(nullptr, nullptr);
  gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scroll), GTK_POLICY_NEVER,
                                 GTK_POLICY_AUTOMATIC);
  gtk_box_pack_start(GTK_BOX(box), scroll, TRUE, TRUE, 0);
  state->grid = gtk_grid_new();
  gtk_grid_set_row_spacing(GTK_GRID(state->grid), 10);
  gtk_grid_set_column_spacing(GTK_GRID(state->grid), 12);
  gtk_container_add(GTK_CONTAINER(scroll), state->grid);
  const auto &c = state->config;
  state->dictionary =
      state->entry("辞書(_D)", "Dictionary", c.dictionary, true);
  state->encoding = state->combo("辞書の文字コード", "DictionaryEncoding",
                                 "euc-jp", "utf-8", c.encoding);
  state->userDictionary = state->entry("ユーザー辞書(_U)", "UserDictionary",
                                       c.userDictionary, true);
  state->inlineCandidates = state->number(
      "一覧を出すまでの候補数", "InlineCandidates", c.inlineCandidates, 0, 64);
  state->indicator =
      state->toggle("モード変更時に表示する(_I)", "ModeIndicator", c.indicator);
  state->ai = state->toggle("AIによる候補の並べ替え(_A)", "ZenzEnabled", c.ai);
  state->model = state->entry("モデル(_M)", "ZenzModel", c.model, true);
  state->server = state->combo("SKKサーバー(_S)", "SkkservMode", "Google",
                               "Custom", c.server, "Disabled");
  state->host = state->entry("カスタムサーバーのホスト", "SkkservHost", c.host);
  state->port = state->number("ポート", "SkkservPort", c.port, 1, 65535);
  state->serverEncoding =
      state->combo("サーバーの文字コード", "SkkservEncoding", "euc-jp", "utf-8",
                   c.serverEncoding);
  state->timeout = state->number("辞書にない候補を待つ時間（ms）",
                                 "SkkservTimeoutMs", c.timeout, 0, 1000);
  auto *privacy = gtk_label_new(
      "Googleにはローカル候補のない読みだけを送り、周辺の文章は送りません。");
  gtk_label_set_line_wrap(GTK_LABEL(privacy), TRUE);
  gtk_label_set_xalign(GTK_LABEL(privacy), 0);
  gtk_box_pack_start(GTK_BOX(box), privacy, FALSE, FALSE, 0);
  state->status = gtk_label_new("");
  gtk_widget_set_name(state->status, "status");
  gtk_label_set_line_wrap(GTK_LABEL(state->status), TRUE);
  gtk_label_set_xalign(GTK_LABEL(state->status), 0);
  gtk_box_pack_start(GTK_BOX(box), state->status, FALSE, FALSE, 0);
  auto *buttons = gtk_button_box_new(GTK_ORIENTATION_HORIZONTAL);
  gtk_button_box_set_layout(GTK_BUTTON_BOX(buttons), GTK_BUTTONBOX_END);
  auto *close = gtk_button_new_with_mnemonic("閉じる(_C)");
  auto *save = gtk_button_new_with_mnemonic("保存(_S)");
  gtk_widget_set_name(save, "save");
  gtk_container_add(GTK_CONTAINER(buttons), close);
  gtk_container_add(GTK_CONTAINER(buttons), save);
  gtk_box_pack_start(GTK_BOX(box), buttons, FALSE, FALSE, 0);
  g_signal_connect_swapped(close, "clicked", G_CALLBACK(gtk_widget_destroy),
                           window);
  g_signal_connect(save, "clicked", G_CALLBACK(+[](GtkButton *, gpointer data) {
                     static_cast<Settings *>(data)->save();
                   }),
                   state.get());
  g_object_set_data_full(
      G_OBJECT(window), "settings", state.release(),
      +[](gpointer data) { delete static_cast<Settings *>(data); });
  return window;
}
} // namespace tgskk_ibus
