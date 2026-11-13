#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <ibus.h>
#include <string>
#include <vector>
using Clock = std::chrono::steady_clock;
std::string preedit, committed;
unsigned commits = 0, tables = 0;
void pump(unsigned ms = 0) {
  auto deadline = Clock::now() + std::chrono::milliseconds(ms);
  do {
    while (g_main_context_pending(nullptr))
      g_main_context_iteration(nullptr, FALSE);
    if (Clock::now() >= deadline)
      break;
    g_usleep(1000);
  } while (true);
}
int main() {
  ibus_init();
  auto *bus = ibus_bus_new();
  g_assert_true(ibus_bus_is_connected(bus));
  auto *ctx =
      ibus_bus_create_input_context(bus, "tgskk-isolated-native-client");
  g_assert_nonnull(ctx);
  g_signal_connect(
      ctx, "update-preedit-text",
      G_CALLBACK(+[](IBusInputContext *, IBusText *t, guint, gboolean visible,
                     gpointer) { preedit = visible ? t->text : ""; }),
      nullptr);
  g_signal_connect(ctx, "commit-text",
                   G_CALLBACK(+[](IBusInputContext *, IBusText *t, gpointer) {
                     committed = t->text;
                     ++commits;
                   }),
                   nullptr);
  g_signal_connect(ctx, "update-lookup-table",
                   G_CALLBACK(+[](IBusInputContext *, IBusLookupTable *t,
                                  gboolean visible, gpointer) {
                     if (visible)
                       tables = ibus_lookup_table_get_number_of_candidates(t);
                   }),
                   nullptr);
  ibus_input_context_set_capabilities(
      ctx, IBUS_CAP_PREEDIT_TEXT | IBUS_CAP_FOCUS | IBUS_CAP_SURROUNDING_TEXT |
               IBUS_CAP_LOOKUP_TABLE | IBUS_CAP_AUXILIARY_TEXT |
               IBUS_CAP_PROPERTY);
  ibus_input_context_focus_in(ctx);
  pump(10);
  auto start = Clock::now();
  g_assert_true(ibus_bus_set_global_engine(bus, "tgskk"));
  ibus_input_context_set_engine(ctx, "tgskk");
  pump(10);
  std::printf(
      "startup_ms %.3f\n",
      std::chrono::duration<double, std::milli>(Clock::now() - start).count());
  auto key = [&](guint symbol, guint mods = 0) {
    auto start = Clock::now();
    ibus_input_context_process_key_event(ctx, symbol, 0, mods);
    auto elapsed =
        std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    pump();
    return elapsed;
  };
  auto reset = [&](std::string left = "") {
    ibus_input_context_reset(ctx);
    ibus_input_context_set_content_type(ctx, IBUS_INPUT_PURPOSE_FREE_FORM, 0);
    ibus_input_context_set_surrounding_text(
        ctx, ibus_text_new_from_string(left.c_str()),
        g_utf8_strlen(left.c_str(), -1), g_utf8_strlen(left.c_str(), -1));
    pump(2);
  };
  auto type = [&](const char *ascii, unsigned interval) {
    while (*ascii) {
      const auto symbol = static_cast<guint>(*ascii++);
      if (symbol >= 'A' && symbol <= 'Z') {
        key(IBUS_KEY_Shift_L);
        key(symbol, IBUS_SHIFT_MASK);
        key(IBUS_KEY_Shift_L, IBUS_SHIFT_MASK | IBUS_RELEASE_MASK);
      } else {
        key(symbol);
      }
      if (interval)
        pump(interval);
    }
  };
  reset();
  type("Kanji ", 0);
  pump(5);
  g_assert_true(preedit == "▼漢字");
  key(IBUS_KEY_Return);
  pump(5);
  g_assert_true(committed == "漢字");
  unsigned before = commits;
  ibus_input_context_reset(ctx);
  pump(2);
  g_assert_cmpuint(commits, ==, before);
  reset();
  type("Ichiran     ", 0);
  pump(2);
  g_assert_cmpuint(tables, >, 0);
  std::puts(
      "Full client/daemon/engine commit, preedit and lookup signals passed.");
  std::fflush(stdout);
  if (g_getenv("TGSKK_TEST_MODEL")) {
    bool ready = false;
    auto deadline = Clock::now() + std::chrono::seconds(7);
    do {
      reset("駅で");
      type("Kisha", 100);
      key(IBUS_KEY_space);
      pump(2);
      ready = preedit == "▼汽車";
    } while (!ready && Clock::now() < deadline);
    g_assert_true(ready);
    for (int i = 0; i < 3; ++i) {
      reset("駅で");
      type("Kisha", 100);
      key(IBUS_KEY_space);
      pump(2);
      g_assert_true(preedit == "▼汽車");
    }
    ibus_input_context_reset(ctx);
    ibus_input_context_set_content_type(ctx, IBUS_INPUT_PURPOSE_FREE_FORM,
                                        IBUS_INPUT_HINT_PRIVATE);
    type("Kisha", 100);
    key(IBUS_KEY_space);
    pump(2);
    g_assert_true(preedit == "▼記者");
  }
  ibus_input_context_focus_out(ctx);
  g_object_unref(ctx);
  g_object_unref(bus);
}
