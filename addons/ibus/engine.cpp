// SPDX-License-Identifier: GPL-3.0-or-later
#include "engine.h"
#include "paths.h"
#include "../common/mode_colors.h"
#include <algorithm>
#include <cstring>
#include <filesystem>
#include <stdexcept>

namespace tgskk_ibus {
std::unique_ptr<Resources> resources;
static std::string errorMessage(char *error) {
  std::string message = error ? error : "tgskk operation failed";
  tgskk_string_free(error);
  return message;
}
Resources::Resources(Config value) : config(std::move(value)) {
  std::filesystem::create_directories(
      std::filesystem::path(config.userDictionary).parent_path());
  char *error = nullptr;
  runtime =
      tgskk_runtime_new(config.dictionary.c_str(), config.encoding.c_str(),
                        config.userDictionary.c_str(), &error);
  if (!runtime) {
    throw std::runtime_error(errorMessage(error));
  }
  if (config.ai) {
    ai = tgskk_zenz_new(TGSKK_WORKER, config.model.c_str(), &error);
    if (!ai) {
      g_warning("%s", errorMessage(error).c_str());
    }
  }
  if (config.server == "Google") {
    server = tgskk_skkserv_google_new(TGSKK_GOOGLE, config.timeout, &error);
  } else if (config.server == "Custom") {
    server = tgskk_skkserv_custom_new(config.host.c_str(), config.port,
                                      config.serverEncoding.c_str(),
                                      config.timeout, &error);
  }
  if (config.server != "Disabled" && !server) {
    g_warning("%s", errorMessage(error).c_str());
  }
}
void Resources::save() {
  char *error = nullptr;
  if (!tgskk_runtime_save(runtime, &error)) {
    g_warning("%s", errorMessage(error).c_str());
  }
}
void Resources::scheduleSave() {
  if (!saveTimer) {
    saveTimer = g_timeout_add_seconds(
        5,
        [](gpointer p) -> gboolean {
          auto *self = static_cast<Resources *>(p);
          self->saveTimer = 0;
          self->save();
          return G_SOURCE_REMOVE;
        },
        this);
  }
}
Resources::~Resources() {
  if (saveTimer) {
    g_source_remove(saveTimer);
  }
  save();
  tgskk_runtime_free(runtime);
  tgskk_zenz_free(ai);
  tgskk_skkserv_free(server);
}
static std::string suffix(const std::string &text) {
  const auto count = g_utf8_strlen(text.c_str(), -1);
  return count > 96 ? g_utf8_offset_to_pointer(text.c_str(), count - 96) : text;
}
static std::string linePrefix(const char *text) {
  const std::string line(text, std::strcspn(text, "\r\n"));
  const auto count = g_utf8_strlen(line.c_str(), -1);
  return count > 32
             ? line.substr(0, g_utf8_offset_to_pointer(line.c_str(), 32) -
                                  line.c_str())
             : line;
}
struct State {
  TgskkSession *session = nullptr;
  bool focused = false, sensitive = false, selection = false, observed = false;
  bool list = false;
  size_t offset = 0;
  guint indicatorTimer = 0;
  std::string left, right, snapshot;
  ~State() {
    if (indicatorTimer) {
      g_source_remove(indicatorTimer);
    }
    tgskk_session_free(session);
  }
  void forget() {
    left.clear();
    right.clear();
    snapshot.clear();
    observed = false;
  }
};
struct TgskkEngine {
  IBusEngine parent;
  State *state;
};
struct TgskkEngineClass {
  IBusEngineClass parent;
};
G_DEFINE_TYPE(TgskkEngine, tgskk_engine, IBUS_TYPE_ENGINE)
static State &state(IBusEngine *engine) {
  return *reinterpret_cast<TgskkEngine *>(engine)->state;
}
static void render(IBusEngine *engine, const TgskkView *view) {
  auto &s = state(engine);
  if (s.indicatorTimer) {
    g_source_remove(s.indicatorTimer);
    s.indicatorTimer = 0;
  }
  if (!view) {
    g_warning("%s", tgskk_session_error(s.session));
    tgskk_session_free(s.session);
    s.session = nullptr;
    ibus_engine_hide_preedit_text(engine);
    ibus_engine_hide_lookup_table(engine);
    ibus_engine_hide_auxiliary_text(engine);
    s.list = false;
    s.forget();
    return;
  }
  if (*view->committed) {
    if (!s.sensitive) {
      s.left = suffix(s.left + view->committed);
    }
    ibus_engine_commit_text(engine, ibus_text_new_from_string(view->committed));
    resources->scheduleSave();
  }
  auto *preedit = ibus_text_new_from_string(view->preedit);
  const auto length = g_utf8_strlen(view->preedit, -1);
  ibus_text_append_attribute(preedit, IBUS_ATTR_TYPE_UNDERLINE,
                             IBUS_ATTR_UNDERLINE_SINGLE, 0, length);
  ibus_engine_update_preedit_text_with_mode(
      engine, preedit, length, length != 0, IBUS_ENGINE_PREEDIT_CLEAR);
  s.list =
      view->selecting && view->candidate_count &&
      view->selected >= static_cast<size_t>(resources->config.inlineCandidates);
  s.offset = s.list ? resources->config.inlineCandidates : 0;
  if (s.list) {
    auto *table = ibus_lookup_table_new(tgskk::candidatePageSize,
                                        view->selected - s.offset, TRUE, FALSE);
    ibus_lookup_table_set_orientation(table, IBUS_ORIENTATION_VERTICAL);
    for (size_t i = s.offset; i < view->candidate_count; ++i) {
      std::string candidate = tgskk_candidate_text(s.session, i);
      if (const char *annotation = tgskk_candidate_annotation(s.session, i)) {
        candidate += " " + std::string(annotation);
      }
      ibus_lookup_table_append_candidate(
          table, ibus_text_new_from_string(candidate.c_str()));
    }
    ibus_engine_update_lookup_table(engine, table, TRUE);
  } else {
    ibus_engine_hide_lookup_table(engine);
  }
  ibus_engine_update_auxiliary_text(
      engine, ibus_text_new_from_string("AI-ranked"),
      s.list && view->zenz_applied && !s.sensitive);
  if (view->mode_changed) {
    auto *property = ibus_property_new(
        "mode", PROP_TYPE_NORMAL, ibus_text_new_from_string(view->mode_label),
        "", ibus_text_new_from_string("Input mode"), TRUE, TRUE,
        PROP_STATE_UNCHECKED, nullptr);
    ibus_engine_update_property(engine, property);
    if (resources->config.indicator && s.focused && !length && !s.list) {
      const char *space = g_unichar_iswide(g_utf8_get_char(view->mode_label))
                              ? "\u2005"
                              : "\u2002";
      // ASCII A is wider than half-width kana in common panel fonts.
      const char *trailing =
          std::strcmp(view->mode_label, "A") == 0 ? "\u2004" : space;
      const auto label = space + std::string(view->mode_label) + trailing;
      auto *text = ibus_text_new_from_string(label.c_str());
      const auto end = g_utf8_strlen(label.c_str(), -1);
      const auto colors = tgskk::modeColors(view->mode_label);
      ibus_text_append_attribute(text, IBUS_ATTR_TYPE_BACKGROUND,
                                 colors.background, 0, end);
      ibus_text_append_attribute(text, IBUS_ATTR_TYPE_FOREGROUND,
                                 colors.foreground, 0, end);
      ibus_engine_update_auxiliary_text(engine, text, TRUE);
      s.indicatorTimer = g_timeout_add(
          700,
          [](gpointer p) -> gboolean {
            auto *e = IBUS_ENGINE(p);
            state(e).indicatorTimer = 0;
            ibus_engine_hide_auxiliary_text(e);
            return G_SOURCE_REMOVE;
          },
          engine);
    }
  }
}
static void reset(IBusEngine *engine) {
  auto &s = state(engine);
  // IBus deduplicates unchanged surrounding text across composition resets.
  if (s.focused && !s.sensitive && !s.selection && s.observed) {
    s.left = s.snapshot;
  } else {
    s.forget();
  }
  if (s.session) {
    render(engine, tgskk_session_reset(s.session));
  }
}
static void context(IBusEngine *engine) {
  auto &s = state(engine);
  const bool allowed = s.focused && !s.sensitive && !s.selection;
  tgskk_session_network_allowed(s.session, allowed);
  if (!allowed || !resources->config.ai) {
    s.forget();
    tgskk_session_context(s.session, nullptr, nullptr, 0);
  } else {
    tgskk_session_context(s.session, s.left.c_str(), s.right.c_str(), 1);
  }
}
static void page(IBusEngine *engine, bool forward, bool registration) {
  auto &s = state(engine);
  const auto *v = tgskk_session_view(s.session);
  if (!v || !v->selecting || !v->candidate_count) {
    return;
  }
  const auto move = tgskk::candidatePageMove(
      v->candidate_count, v->selected,
      static_cast<size_t>(resources->config.inlineCandidates), forward,
      registration);
  if (move.kind == tgskk::CandidateMoveKind::Stay)
    return;
  v = tgskk_session_choose(s.session, move.index);
  if (v && move.kind == tgskk::CandidateMoveKind::Register)
    v = tgskk_session_key(s.session, IBUS_KEY_space, 0, 0);
  else if (v && move.kind == tgskk::CandidateMoveKind::ReturnToReading)
    v = tgskk_session_key(s.session, IBUS_KEY_x, 0, 0);
  render(engine, v);
}
static bool initialize(State &s) {
  if (s.session) {
    return true;
  }
  char *error = nullptr;
  s.session = tgskk_session_new(resources->runtime, &error);
  if (!s.session) {
    g_warning("%s", errorMessage(error).c_str());
    return false;
  }
  if (!tgskk_session_set_zenz(s.session, resources->ai) ||
      !tgskk_session_set_skkserv(s.session, resources->server)) {
    g_warning("%s", tgskk_session_error(s.session));
    tgskk_session_free(s.session);
    s.session = nullptr;
    return false;
  }
  return true;
}
static bool modifierKey(guint symbol) {
  if (symbol >= IBUS_KEY_Shift_L && symbol <= IBUS_KEY_Hyper_R) {
    return true;
  }
  switch (symbol) {
  case IBUS_KEY_Num_Lock:
  case IBUS_KEY_Scroll_Lock:
  case IBUS_KEY_Mode_switch:
  case IBUS_KEY_ISO_Level3_Shift:
  case IBUS_KEY_ISO_Level3_Latch:
  case IBUS_KEY_ISO_Level3_Lock:
  case IBUS_KEY_ISO_Level5_Shift:
  case IBUS_KEY_ISO_Level5_Latch:
  case IBUS_KEY_ISO_Level5_Lock:
    return true;
  default:
    return false;
  }
}
static gboolean key(IBusEngine *engine, guint symbol, guint, guint modifiers) {
  auto &s = state(engine);
  if ((modifiers & IBUS_RELEASE_MASK) || modifierKey(symbol)) {
    return FALSE;
  }
  if (!initialize(s)) {
    return FALSE;
  }
  context(engine);
  for (char *error : {tgskk_zenz_take_error(resources->ai),
                      tgskk_skkserv_take_error(resources->server)}) {
    if (error) {
      g_warning("%s", error);
      tgskk_string_free(error);
    }
  }
  const auto *v = tgskk_session_view(s.session);
  // IBus adds transport flags; only XKB modifier bits belong in the core.
  if (modifiers & IBUS_HYPER_MASK) {
    modifiers |= IBUS_MOD3_MASK;
  }
  modifiers &= IBUS_SHIFT_MASK | IBUS_CONTROL_MASK | IBUS_MOD1_MASK |
               IBUS_MOD3_MASK | IBUS_MOD4_MASK | IBUS_SUPER_MASK |
               IBUS_META_MASK;
  if (v && v->selecting && !modifiers) {
    if (symbol == IBUS_KEY_Page_Down || symbol == IBUS_KEY_Page_Up ||
        (s.list && (symbol == IBUS_KEY_space || symbol == IBUS_KEY_x))) {
      if (s.list || symbol == IBUS_KEY_Page_Down) {
        page(engine, symbol == IBUS_KEY_space || symbol == IBUS_KEY_Page_Down,
             symbol == IBUS_KEY_space);
      }
      return TRUE;
    }
    if (symbol == IBUS_KEY_Down || symbol == IBUS_KEY_Up) {
      const auto target = tgskk::candidateStep(v->candidate_count, v->selected,
                                               symbol == IBUS_KEY_Down);
      render(engine, tgskk_session_choose(s.session, target));
      return TRUE;
    }
    if (s.list && symbol >= IBUS_KEY_1 &&
        symbol < IBUS_KEY_1 + tgskk::candidatePageSize) {
      const auto index = tgskk::candidatePageStart(v->selected, s.offset) +
                         symbol - IBUS_KEY_1;
      if (index < v->candidate_count) {
        render(engine, tgskk_session_select(s.session, index));
      }
      return TRUE;
    }
  }
  v = tgskk_session_key(s.session, symbol, modifiers, 0);
  const bool consumed = !v || v->consumed;
  render(engine, v);
  if (consumed && symbol == IBUS_KEY_X) {
    resources->scheduleSave();
  }
  if (!consumed) {
    s.forget();
    tgskk_session_context(s.session, nullptr, nullptr, 0);
  }
  return consumed;
}
static void focusIn(IBusEngine *engine) {
  auto &s = state(engine);
  s.focused = false;
  s.forget();
  reset(engine);
  s.focused = true;
  s.selection = false;
  auto *properties = ibus_prop_list_new();
  const auto *view = tgskk_session_view(s.session);
  ibus_prop_list_append(
      properties, ibus_property_new(
                      "mode", PROP_TYPE_NORMAL,
                      ibus_text_new_from_string(view ? view->mode_label : "あ"),
                      "", ibus_text_new_from_string("Input mode"), TRUE, TRUE,
                      PROP_STATE_UNCHECKED, nullptr));
  ibus_engine_register_properties(engine, properties);
  ibus_engine_get_surrounding_text(engine, nullptr, nullptr, nullptr);
}
static void focusOut(IBusEngine *engine) {
  state(engine).focused = false;
  reset(engine);
  resources->scheduleSave();
}
static void surrounding(IBusEngine *engine, IBusText *text, guint cursor,
                        guint anchor) {
  auto &s = state(engine);
  const auto count = g_utf8_strlen(text->text, -1);
  if (cursor > count || anchor > count) {
    s.forget();
    s.selection = true;
    context(engine);
    return;
  }
  s.selection = cursor != anchor;
  if (s.sensitive || !s.focused || s.selection) {
    s.forget();
  } else {
    const auto *caret = g_utf8_offset_to_pointer(text->text, cursor);
    const auto left = suffix(std::string(text->text, caret - text->text));
    // Committing inserts at the caret, so the right side stays current.
    s.right = linePrefix(caret);
    if (!s.observed || s.snapshot != left) {
      s.left = left;
      s.snapshot = left;
      s.observed = true;
    }
  }
  context(engine);
}
static void content(IBusEngine *engine, guint purpose, guint hints) {
  auto &s = state(engine);
  const bool wasSensitive = s.sensitive;
  // HIDDEN_TEXT is bit 12 since IBus 1.5.34; also honor it with older headers.
  s.sensitive = purpose == IBUS_INPUT_PURPOSE_PASSWORD ||
                purpose == IBUS_INPUT_PURPOSE_PIN ||
                (hints & (IBUS_INPUT_HINT_PRIVATE | (1u << 12)));
  if (s.sensitive) {
    reset(engine);
  }
  if (wasSensitive != s.sensitive) {
    s.forget();
  }
  context(engine);
}
static void clicked(IBusEngine *engine, guint index, guint button, guint) {
  auto &s = state(engine);
  if (!s.focused || !s.list || !s.session || button != 1) {
    return;
  }
  context(engine);
  const auto *v = tgskk_session_view(s.session);
  if (!v || !v->selecting || index >= tgskk::candidatePageSize) {
    return;
  }
  const auto target = tgskk::candidatePageStart(v->selected, s.offset) + index;
  if (target < v->candidate_count) {
    render(engine, tgskk_session_select(s.session, target));
  }
}
static void destroy(IBusObject *object) {
  auto *engine = reinterpret_cast<TgskkEngine *>(object);
  delete engine->state;
  engine->state = nullptr;
  IBUS_OBJECT_CLASS(tgskk_engine_parent_class)->destroy(object);
}
static void tgskk_engine_init(TgskkEngine *engine) {
  engine->state = new State;
  initialize(*engine->state);
}
static void tgskk_engine_class_init(TgskkEngineClass *klass) {
  auto *engine = IBUS_ENGINE_CLASS(klass);
  engine->process_key_event = key;
  engine->focus_in = focusIn;
  engine->focus_out = focusOut;
  engine->enable = [](IBusEngine *e) {
    ibus_engine_get_surrounding_text(e, nullptr, nullptr, nullptr);
  };
  engine->disable = focusOut;
  engine->reset = reset;
  engine->set_capabilities = [](IBusEngine *e, guint caps) {
    e->client_capabilities = caps;
    if (!(caps & IBUS_CAP_SURROUNDING_TEXT)) {
      state(e).forget();
      state(e).selection = false;
      context(e);
    }
  };
  engine->set_surrounding_text = surrounding;
  engine->set_content_type = content;
  engine->candidate_clicked = clicked;
  engine->page_down = [](IBusEngine *e) { page(e, true, false); };
  engine->page_up = [](IBusEngine *e) { page(e, false, false); };
  engine->cursor_down = [](IBusEngine *e) { key(e, IBUS_KEY_Down, 0, 0); };
  engine->cursor_up = [](IBusEngine *e) { key(e, IBUS_KEY_Up, 0, 0); };
  IBUS_OBJECT_CLASS(klass)->destroy = destroy;
}
GType engineType() { return tgskk_engine_get_type(); }
} // namespace tgskk_ibus
