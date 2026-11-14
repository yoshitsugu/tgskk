// SPDX-License-Identifier: GPL-3.0-or-later
#include "tgskk.h"
#include "../common/candidates.h"
#include "paths.h"
#include "tgskk-features.h"

#include <fcitx-config/configuration.h>
#include <fcitx-config/iniparser.h>
#include <fcitx-config/option.h>
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/userinterface.h>
#include <fcitx/surroundingtext.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <filesystem>
#include <memory>
#include <string>

namespace {
using namespace fcitx;

static_assert(static_cast<uint32_t>(TGSKK_KEY_BACKSPACE) == FcitxKey_BackSpace);
static_assert(static_cast<uint32_t>(TGSKK_KEY_TAB) == FcitxKey_Tab);
static_assert(static_cast<uint32_t>(TGSKK_KEY_RETURN) == FcitxKey_Return);
static_assert(static_cast<uint32_t>(TGSKK_KEY_ESCAPE) == FcitxKey_Escape);
static_assert(static_cast<uint32_t>(TGSKK_KEY_LEFT) == FcitxKey_Left);
static_assert(static_cast<uint32_t>(TGSKK_KEY_UP) == FcitxKey_Up);
static_assert(static_cast<uint32_t>(TGSKK_KEY_RIGHT) == FcitxKey_Right);
static_assert(static_cast<uint32_t>(TGSKK_KEY_DOWN) == FcitxKey_Down);
static_assert(static_cast<uint32_t>(TGSKK_KEY_PAGE_UP) == FcitxKey_Page_Up);
static_assert(static_cast<uint32_t>(TGSKK_KEY_PAGE_DOWN) == FcitxKey_Page_Down);
static_assert(TGSKK_MOD_SHIFT == static_cast<uint32_t>(KeyState::Shift));
static_assert(TGSKK_MOD_CONTROL == static_cast<uint32_t>(KeyState::Ctrl));
static_assert(TGSKK_MOD_ALT == static_cast<uint32_t>(KeyState::Alt));

struct EncodingConstrain {
  using Type = std::string;
  static constexpr std::array<const char *, 2> values{"euc-jp", "utf-8"};
  bool check(const std::string &value) const {
    return std::find(values.begin(), values.end(), value) != values.end();
  }
  void dumpDescription(RawConfig &config) const {
    config.setValueByPath("IsEnum", "True");
    for (size_t i = 0; i < values.size(); ++i) {
      config.setValueByPath("Enum/" + std::to_string(i), values[i]);
    }
  }
};

struct SkkservModeConstrain {
  using Type = std::string;
  static constexpr std::array<const char *, 3> values{"Google", "Custom",
                                                      "Disabled"};
  bool check(const std::string &value) const {
    return std::find(values.begin(), values.end(), value) != values.end();
  }
  void dumpDescription(RawConfig &config) const {
    config.setValueByPath("IsEnum", "True");
    for (size_t i = 0; i < values.size(); ++i) {
      config.setValueByPath("Enum/" + std::to_string(i), values[i]);
    }
  }
};

FCITX_CONFIGURATION(
    TgskkConfig,
    Option<std::string> dictionary{this, "Dictionary", "SKK dictionary path",
                                   "/usr/share/skk/SKK-JISYO.L"};
    Option<std::string, EncodingConstrain> encoding{
        this, "Encoding", "Dictionary encoding", "euc-jp"};
    Option<int, IntConstrain> inlineCandidates{
        this, "InlineCandidates", "Candidates before showing the list",
        static_cast<int>(tgskk::defaultInlineCandidates), IntConstrain(0)};
    Option<bool> modeIndicator{this, "ModeIndicator",
                               "Show mode near the cursor", true};
    Option<std::string, SkkservModeConstrain> skkservMode{
        this, "SkkservMode", "Online candidates when no local candidate exists",
        "Google"};
    Option<std::string> skkservHost{this, "SkkservHost",
                                    "Custom SKK server host", "127.0.0.1"};
    Option<int, IntConstrain> skkservPort{this, "SkkservPort",
                                          "Custom SKK server port", 1178,
                                          IntConstrain(1, 65535)};
    Option<std::string, EncodingConstrain> skkservEncoding{
        this, "SkkservEncoding", "Custom SKK server encoding", "euc-jp"};
    Option<int, IntConstrain> skkservTimeoutMs{
        this, "SkkservTimeoutMs", "Wait for missing candidates (ms)", 300,
        IntConstrain(0, 1000)};
    Option<bool> zenzEnabled{this, "ZenzEnabled", "Use local Zenz ranking",
                             true};
    Option<std::string> zenzModel{this, "ZenzModel", "Zenz v3 model path",
                                  TGSKK_ZENZ_MODEL};);

using Zenz = std::unique_ptr<TgskkZenz, decltype(&tgskk_zenz_free)>;
using Skkserv = std::unique_ptr<TgskkSkkserv, decltype(&tgskk_skkserv_free)>;
using Runtime = std::unique_ptr<TgskkRuntime, decltype(&tgskk_runtime_free)>;
using Session = std::unique_ptr<TgskkSession, decltype(&tgskk_session_free)>;

std::string takeError(char *message) {
  std::string result = message ? message : "Unknown tgskk error";
  tgskk_string_free(message);
  return result;
}

std::string modelPath(const std::string &configured) {
  if (configured != TGSKK_ZENZ_MODEL) {
    return configured;
  }
  const auto userModel = tgskk::userDataDirectory() / "tgskk" /
                         std::filesystem::path(TGSKK_ZENZ_MODEL).filename();
  std::error_code error;
  return std::filesystem::is_regular_file(userModel, error) ? userModel.string()
                                                            : configured;
}

class TgskkState : public InputContextProperty {
public:
  Session session{nullptr, tgskk_session_free};
  uint64_t generation = 0;
  size_t candidateOffset = 0;
  std::string left;
  std::string observedLeft;
  bool observed = false;
  bool sensitive = false;
  void forgetContext() {
    left.clear();
    observedLeft.clear();
    observed = false;
  }
};

class TgskkEngine;

class TgskkCandidate : public CandidateWord {
public:
  TgskkCandidate(TgskkEngine *engine, size_t index, uint64_t generation,
                 const char *text, const char *annotation)
      : CandidateWord(Text(text)), engine_(engine), index_(index),
        generation_(generation) {
    if (annotation) {
#if TGSKK_FCITX_HAS_COMMENT
      setComment(Text(annotation));
#else
      Text display(text);
      display.append(" " + std::string(annotation), TextFormatFlag::Italic);
      setText(std::move(display));
#endif
    }
  }
  void select(InputContext *ic) const override;

private:
  TgskkEngine *engine_;
  size_t index_;
  uint64_t generation_;
};

class TgskkEngine : public InputMethodEngineV2 {
public:
  explicit TgskkEngine(Instance *instance)
      : instance_(instance),
        factory_([](InputContext &) { return new TgskkState; }) {
    instance_->inputContextManager().registerProperty("tgskk", &factory_);
    reloadConfig();
  }

  void keyEvent(const InputMethodEntry &, KeyEvent &event) override {
    auto *ic = event.inputContext();
    auto *state = ic->propertyFor(&factory_);
    if (event.isRelease()) {
      return;
    }
    if (!state->session && runtime_) {
      initialize(ic);
    }
    if (!state->session) {
      return;
    }
    if (auto *message = tgskk_zenz_take_error(zenz_.get())) {
      FCITX_ERROR() << "tgskk: " << takeError(message);
    }
    if (auto *message = tgskk_skkserv_take_error(skkserv_.get())) {
      FCITX_ERROR() << "tgskk: " << takeError(message);
    }
    updateContext(ic);
    auto *session = state->session.get();
    auto *view = tgskk_session_view(session);
    auto list = std::dynamic_pointer_cast<CommonCandidateList>(
        ic->inputPanel().candidateList());
    if (view && view->selecting && list) {
      // Mouse wheel paging may move the UI cursor before a key reaches us.
      const auto cursor = list->globalCursorIndex();
      if (cursor >= 0 && state->candidateOffset + static_cast<size_t>(cursor) !=
                             view->selected) {
        view = tgskk_session_choose(session, state->candidateOffset +
                                                 static_cast<size_t>(cursor));
        if (!view) {
          failed(ic);
          event.filterAndAccept();
          return;
        }
      }
      const bool next = event.key().check(Key(FcitxKey_space)) ||
                        event.key().check(Key(FcitxKey_Page_Down));
      const bool previous = event.key().check(Key(FcitxKey_x)) ||
                            event.key().check(Key(FcitxKey_Page_Up));
      if (next || previous) {
        movePage(ic, view, *list, next, event.key().check(Key(FcitxKey_space)));
        event.filterAndAccept();
        return;
      }
      const auto index = event.key().keyListIndex(selectionKeys_);
      if (index >= 0 && index < list->size()) {
        list->candidate(index).select(ic);
        event.filterAndAccept();
        return;
      }
    }
    if (view && view->selecting && !list &&
        (event.key().check(Key(FcitxKey_Page_Down)) ||
         event.key().check(Key(FcitxKey_Page_Up)))) {
      // Keep page navigation inside the composition while the list is hidden.
      if (event.key().check(Key(FcitxKey_Page_Down))) {
        const auto target =
            tgskk::candidatePageMove(
                view->candidate_count, view->selected,
                static_cast<size_t>(*config_.inlineCandidates), true)
                .index;
        render(ic, tgskk_session_choose(session, target));
      }
      event.filterAndAccept();
      return;
    }
    if (view && view->selecting &&
        (event.key().check(Key(FcitxKey_Down)) ||
         event.key().check(Key(FcitxKey_Up)))) {
      const auto target =
          tgskk::candidateStep(view->candidate_count, view->selected,
                               event.key().check(Key(FcitxKey_Down)));
      if (target != view->selected) {
        render(ic, tgskk_session_choose(session, target));
      }
      event.filterAndAccept();
      return;
    }
    uint32_t symbol = event.rawKey().sym();
    uint32_t modifiers =
        static_cast<uint32_t>(event.rawKey().states() & KeyState::SimpleMask);
    view = tgskk_session_key(session, symbol, modifiers, 0);
    if (!view || view->consumed) {
      event.filterAndAccept();
    }
    render(ic, view);
    if (view && !view->consumed) {
      state->forgetContext();
      tgskk_session_context(session, nullptr, 0);
      tgskk_session_network_allowed(session, 0);
    }
  }

  void activate(const InputMethodEntry &, InputContextEvent &event) override {
    auto *ic = event.inputContext();
    initialize(ic);
  }

  void initialize(InputContext *ic) {
    auto *state = ic->propertyFor(&factory_);
    if (!state->session && runtime_) {
      char *error = nullptr;
      state->session.reset(tgskk_session_new(runtime_.get(), &error));
      if (!state->session) {
        error_ = takeError(error);
      } else if (!tgskk_session_set_zenz(state->session.get(), zenz_.get())) {
        state->session.reset();
        error_ = "Could not attach Zenz worker";
      } else if (!tgskk_session_set_skkserv(state->session.get(),
                                            skkserv_.get())) {
        state->session.reset();
        error_ = "Could not attach SKK server worker";
      }
    }
    if (!state->session) {
      showInformation(ic, "tgskk: " + error_);
      return;
    }
    render(ic, tgskk_session_view(state->session.get()), false, false);
  }

  void reset(const InputMethodEntry &, InputContextEvent &event) override {
    clear(event.inputContext());
  }
  void deactivate(const InputMethodEntry &, InputContextEvent &event) override {
    clear(event.inputContext());
  }

  std::string subModeLabelImpl(const InputMethodEntry &,
                               InputContext &ic) override {
    auto *state = ic.propertyFor(&factory_);
    auto *view = tgskk_session_view(state->session.get());
    return view ? view->mode_label : "!";
  }

  const Configuration *getConfig() const override { return &config_; }

  void setConfig(const RawConfig &raw) override {
    TgskkConfig next = config_;
    next.load(raw, true);
    configure(next, true);
  }

  void reloadConfig() override {
    TgskkConfig next;
    readAsIni(next, "conf/tgskk.conf");
    configure(next, false);
  }

  void save() override { saveRuntime(); }

  bool saveRuntime() {
    if (runtime_) {
      char *error = nullptr;
      if (!tgskk_runtime_save(runtime_.get(), &error)) {
        error_ = takeError(error);
        FCITX_ERROR() << "tgskk: " << error_;
        return false;
      }
    }
    return true;
  }

  void confirm(InputContext *ic, size_t index, uint64_t generation) {
    auto *state = ic->propertyFor(&factory_);
    if (!ic->hasFocus() || !state->session || generation != state->generation ||
        instance_->inputMethod(ic) != "tgskk") {
      return;
    }
    updateContext(ic);
    render(ic, tgskk_session_select(state->session.get(), index));
  }

private:
  void showInformation(InputContext *ic, const std::string &message,
                       [[maybe_unused]] bool mode = false) {
#if TGSKK_FCITX_HAS_CUSTOM_INFORMATION
    instance_->showCustomInputMethodInformation(ic, message);
#else
    // Older fcitx displays the engine's sub-mode label with its own timeout.
    if (mode) {
      instance_->showInputMethodInformation(ic);
    } else {
      ic->inputPanel().setAuxUp(Text(message));
      ic->updateUserInterface(UserInterfaceComponent::InputPanel);
    }
#endif
  }

  void movePage(InputContext *ic, const TgskkView *view,
                const CommonCandidateList &list, bool forward,
                bool registerAtEnd) {
    auto *state = ic->propertyFor(&factory_);
    auto *session = state->session.get();
    const auto page = static_cast<size_t>(list.currentPage());
    const auto pageSize = static_cast<size_t>(list.pageSize());
    const auto move = tgskk::candidatePageMove(
        view->candidate_count, state->candidateOffset + page * pageSize,
        state->candidateOffset, forward, registerAtEnd, pageSize);
    if (move.kind == tgskk::CandidateMoveKind::Stay)
      return;
    view = tgskk_session_choose(session, move.index);
    if (view && move.kind == tgskk::CandidateMoveKind::Register)
      view = tgskk_session_key(session, FcitxKey_space, 0, 0);
    else if (view && move.kind == tgskk::CandidateMoveKind::ReturnToReading)
      view = tgskk_session_key(session, FcitxKey_x, 0, 0);
    render(ic, view);
  }

  static std::string suffix(const std::string &text) {
    const auto length = utf8::length(text);
    return length > 96
               ? text.substr(utf8::ncharByteLength(text.begin(), length - 96))
               : text;
  }

  void updateContext(InputContext *ic) {
    auto *state = ic->propertyFor(&factory_);
    const auto &surrounding = ic->surroundingText();
    const auto caps = ic->capabilityFlags();
    state->sensitive = caps.testAny(CapabilityFlag::PasswordOrSensitive) ||
                       caps.test(CapabilityFlag::Disable);
    const bool selection =
        surrounding.isValid() && surrounding.cursor() != surrounding.anchor();
    const bool allowed = !state->sensitive && !selection && ic->hasFocus();
    tgskk_session_network_allowed(state->session.get(), allowed);
    if (!allowed || !*config_.zenzEnabled) {
      state->forgetContext();
      tgskk_session_context(state->session.get(), nullptr, 0);
      return;
    }
    if (!surrounding.isValid() && state->observed) {
      state->forgetContext();
    }
    if (surrounding.isValid()) {
      const auto left = suffix(surrounding.text().substr(
          0, utf8::ncharByteLength(surrounding.text().begin(),
                                   surrounding.cursor())));
      // Applications may acknowledge commitString after subsequent keys arrive.
      // An unchanged snapshot must not discard our newer local commits.
      if (!state->observed || state->observedLeft != left) {
        state->left = left;
        state->observedLeft = left;
        state->observed = true;
      }
    }
    tgskk_session_context(state->session.get(), state->left.c_str(), 1);
  }

  void configure(const TgskkConfig &next, bool persist) {
    const bool replaceServer =
        !configured_ || *next.skkservMode != *config_.skkservMode ||
        *next.skkservHost != *config_.skkservHost ||
        *next.skkservPort != *config_.skkservPort ||
        *next.skkservEncoding != *config_.skkservEncoding ||
        *next.skkservTimeoutMs != *config_.skkservTimeoutMs;
    Skkserv server(nullptr, tgskk_skkserv_free);
    if (replaceServer && *next.skkservMode != "Disabled") {
      char *error = nullptr;
      if (*next.skkservMode == "Google") {
        server.reset(tgskk_skkserv_google_new(TGSKK_GOOGLE_SKKSERV,
                                              *next.skkservTimeoutMs, &error));
      } else if (*next.skkservMode == "Custom") {
        server.reset(tgskk_skkserv_custom_new(
            next.skkservHost->c_str(), static_cast<uint16_t>(*next.skkservPort),
            next.skkservEncoding->c_str(), *next.skkservTimeoutMs, &error));
      }
      if (!server) {
        error_ = error ? takeError(error) : "Invalid SKK server mode";
        FCITX_ERROR() << "tgskk: " << error_;
        return;
      }
    }
    const bool replaceDictionary = !runtime_ ||
                                   *next.dictionary != *config_.dictionary ||
                                   *next.encoding != *config_.encoding;
    Runtime replacement(nullptr, tgskk_runtime_free);
    if (replaceDictionary) {
      if (!saveRuntime()) {
        return;
      }
      auto userPath = tgskk::userDataDirectory() / "tgskk/SKK-JISYO.user";
      std::error_code ec;
      std::filesystem::create_directories(userPath.parent_path(), ec);
      char *error = nullptr;
      if (ec) {
        error_ = ec.message();
      } else {
        replacement.reset(tgskk_runtime_new(next.dictionary->c_str(),
                                            next.encoding->c_str(),
                                            userPath.c_str(), &error));
        if (!replacement) {
          error_ = takeError(error);
        }
      }
      if (!replacement) {
        FCITX_ERROR() << "tgskk: " << error_;
        return;
      }
    }
    if (persist && !safeSaveAsIni(next, "conf/tgskk.conf")) {
      error_ = "Could not save configuration";
      FCITX_ERROR() << "tgskk: " << error_;
      return;
    }
    const bool replaceZenz = !configured_ ||
                             *next.zenzEnabled != *config_.zenzEnabled ||
                             *next.zenzModel != *config_.zenzModel;
    const bool changeCandidateDisplay =
        *next.inlineCandidates != *config_.inlineCandidates;
    config_ = next;
    configured_ = true;
    if (replaceServer) {
      skkserv_ = std::move(server);
      instance_->inputContextManager().foreach ([this](InputContext *ic) {
        auto *state = ic->propertyFor(&factory_);
        if (state->session &&
            !tgskk_session_set_skkserv(state->session.get(), skkserv_.get())) {
          failed(ic);
        }
        return true;
      });
    }
    if (replaceZenz) {
      zenz_.reset();
      if (*config_.zenzEnabled) {
        char *error = nullptr;
        const auto model = modelPath(*config_.zenzModel);
        zenz_.reset(tgskk_zenz_new(TGSKK_ZENZ_WORKER, model.c_str(), &error));
        if (!zenz_) {
          FCITX_ERROR() << "tgskk: " << takeError(error);
        }
      }
      instance_->inputContextManager().foreach ([this](InputContext *ic) {
        auto *state = ic->propertyFor(&factory_);
        state->forgetContext();
        if (state->session &&
            !tgskk_session_set_zenz(state->session.get(), zenz_.get())) {
          failed(ic);
        }
        return true;
      });
    }
    error_.clear();
    if (!replaceDictionary) {
      if (changeCandidateDisplay) {
        instance_->inputContextManager().foreach ([this](InputContext *ic) {
          auto *state = ic->propertyFor(&factory_);
          if (state->session && instance_->inputMethod(ic) == "tgskk") {
            render(ic, tgskk_session_view(state->session.get()), false, false);
          }
          return true;
        });
      }
      return;
    }
    runtime_ = std::move(replacement);
    instance_->inputContextManager().foreach ([this](InputContext *ic) {
      auto *state = ic->propertyFor(&factory_);
      if (state->session) {
        state->session.reset();
        state->generation = ++generation_;
        if (instance_->inputMethod(ic) == "tgskk") {
          ic->inputPanel().reset();
          ic->updatePreedit();
          ic->updateUserInterface(UserInterfaceComponent::InputPanel);
          initialize(ic);
        }
      }
      return true;
    });
  }

  void clear(InputContext *ic) {
    auto *state = ic->propertyFor(&factory_);
    state->generation = ++generation_;
    state->forgetContext();
    if (state->session && !tgskk_session_reset(state->session.get())) {
      failed(ic);
      return;
    }
    ic->inputPanel().reset();
    ic->updatePreedit();
    ic->updateUserInterface(UserInterfaceComponent::InputPanel);
    ic->updateUserInterface(UserInterfaceComponent::StatusArea);
  }

  void failed(InputContext *ic) {
    auto *state = ic->propertyFor(&factory_);
    const std::string message = tgskk_session_error(state->session.get());
    FCITX_ERROR() << "tgskk: " << message;
    state->session.reset();
    state->generation = ++generation_;
    ic->inputPanel().reset();
    ic->updatePreedit();
    ic->updateUserInterface(UserInterfaceComponent::InputPanel);
    ic->updateUserInterface(UserInterfaceComponent::StatusArea);
    if (ic->hasFocus()) {
      showInformation(ic, "tgskk: " + message);
    }
  }

  void render(InputContext *ic, const TgskkView *view, bool showMode = true,
              bool commit = true) {
    if (!view) {
      failed(ic);
      return;
    }
    auto *state = ic->propertyFor(&factory_);
    state->generation = ++generation_;
    auto &panel = ic->inputPanel();
    panel.reset();
    if (commit && *view->committed) {
      if (!state->sensitive) {
        state->left = suffix(state->left + view->committed);
      }
      ic->commitString(view->committed);
    }
    const auto inlineCount = static_cast<size_t>(*config_.inlineCandidates);
    const bool showCandidates = view->selecting && view->candidate_count &&
                                view->selected >= inlineCount;
    state->candidateOffset = showCandidates ? inlineCount : 0;
    if (*config_.zenzEnabled && !state->sensitive && showCandidates) {
      if (view->zenz_applied) {
        panel.setAuxUp(Text("AI-ranked", TextFormatFlag::Italic));
      } else if (tgskk_zenz_status(zenz_.get()) == 2) {
        panel.setAuxUp(Text("Zenz unavailable — dictionary order"));
      }
    }
    Text preedit(view->preedit, TextFormatFlags{TextFormatFlag::Underline,
                                                TextFormatFlag::DontCommit});
    preedit.setCursor(static_cast<int>(preedit.toString().size()));
    if (ic->capabilityFlags().test(CapabilityFlag::Preedit)) {
      panel.setClientPreedit(preedit);
    } else {
      panel.setPreedit(preedit);
    }
    if (showCandidates) {
      auto list = std::make_unique<CommonCandidateList>();
      list->setSelectionKey(selectionKeys_);
      list->setPageSize(static_cast<int>(selectionKeys_.size()));
      list->setCursorPositionAfterPaging(
          CursorPositionAfterPaging::ResetToFirst);
      list->setLayoutHint(CandidateLayoutHint::Vertical);
      for (size_t i = state->candidateOffset; i < view->candidate_count; ++i) {
        list->append<TgskkCandidate>(
            this, i, state->generation,
            tgskk_candidate_text(state->session.get(), i),
            tgskk_candidate_annotation(state->session.get(), i));
      }
      const auto cursor = view->selected - state->candidateOffset;
      list->setPage(static_cast<int>(cursor / selectionKeys_.size()));
      list->setGlobalCursorIndex(static_cast<int>(cursor));
      panel.setCandidateList(std::move(list));
    }
    ic->updatePreedit();
    ic->updateUserInterface(UserInterfaceComponent::InputPanel);
    if (view->mode_changed) {
      ic->updateUserInterface(UserInterfaceComponent::StatusArea);
      if (showMode && *config_.modeIndicator && ic->hasFocus()) {
        showInformation(ic, view->mode_label, true);
      }
    }
  }

  Instance *instance_;
  TgskkConfig config_;
  Runtime runtime_{nullptr, tgskk_runtime_free};
  Zenz zenz_{nullptr, tgskk_zenz_free};
  Skkserv skkserv_{nullptr, tgskk_skkserv_free};
  bool configured_ = false;
  std::string error_;
  uint64_t generation_ = 0;
  FactoryFor<TgskkState> factory_;
  const KeyList selectionKeys_{
      Key(FcitxKey_1), Key(FcitxKey_2), Key(FcitxKey_3), Key(FcitxKey_4),
      Key(FcitxKey_5), Key(FcitxKey_6), Key(FcitxKey_7)};
};

void TgskkCandidate::select(InputContext *ic) const {
  engine_->confirm(ic, index_, generation_);
}

class TgskkFactory : public AddonFactory {
  AddonInstance *create(AddonManager *manager) override {
    return new TgskkEngine(manager->instance());
  }
};
} // namespace

#ifdef FCITX_ADDON_FACTORY_V2
FCITX_ADDON_FACTORY_V2(tgskk, TgskkFactory)
#else
FCITX_ADDON_FACTORY(TgskkFactory)
#endif
